//! Translation of `dictBuilder/cover.h` (shared types) and
//! `dictBuilder/cover.c` (the COVER dictionary builder).
//!
//! Build configuration notes:
//!   * `ZSTD_MULTITHREAD` is **not** defined, so the `ZSTD_pthread_*` wrappers
//!     from `common/threading.h` are the no-op `int` variants.  This keeps
//!     `COVER_best_t`'s layout identical to the C build.
//!   * The target is linux/glibc, so `_GNU_SOURCE` is defined: `stableSort()`
//!     uses glibc's 5-argument `qsort_r()` and the `g_coverCtx` C90 fallback
//!     global does not exist.
//!   * `DISPLAY`/`DISPLAYLEVEL`/`LOCALDISPLAYLEVEL` only ever `fprintf` to
//!     stderr, so they are dropped; `g_displayLevel` and every assignment to it
//!     are kept because control flow reads it.  `LOCALDISPLAYUPDATE` also
//!     touches `g_time`/`clock()`, which is reproduced faithfully.
//!   * `assert()` is compiled out.
#![allow(dead_code)]

use crate::common::bits::ZSTD_highbit32;
use crate::common::error_private::{
    ERROR, ERR_isError, ZSTD_error_GENERIC, ZSTD_error_dstSize_tooSmall,
    ZSTD_error_memory_allocation, ZSTD_error_parameter_outOfBound, ZSTD_error_srcSize_wrong,
};
use crate::common::mem::{MEM_readLE64, BYTE, U32, U64};
use crate::common::pool::{POOL_add, POOL_create, POOL_ctx, POOL_free};
use crate::common::zstd_internal::{MAX, MIN};
use crate::libc::{clock, free, malloc, memcmp, memcpy, memset, qsort_r};
use core::ffi::{c_int, c_uint, c_void};
use core::mem::size_of;
use core::ptr;

/*-*************************************
*  `zdict.h` types
***************************************/

/// `ZDICT_params_t`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZDICT_params_t {
    pub compressionLevel: c_int,
    pub notificationLevel: c_uint,
    pub dictID: c_uint,
}

/// `ZDICT_cover_params_t`
#[repr(C)]
#[derive(Copy, Clone)]
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

/// `#define ZDICT_DICTSIZE_MIN 256`
pub const ZDICT_DICTSIZE_MIN: usize = 256;
/// `#define ZDICT_CONTENTSIZE_MIN 128`
pub const ZDICT_CONTENTSIZE_MIN: usize = 128;

/*-*************************************
*  `common/threading.h`, ZSTD_MULTITHREAD undefined
***************************************/

/// `typedef int ZSTD_pthread_mutex_t;`
pub type ZSTD_pthread_mutex_t = c_int;
/// `typedef int ZSTD_pthread_cond_t;`
pub type ZSTD_pthread_cond_t = c_int;
/// `ZSTD_pthread_t` is not usable without `ZSTD_MULTITHREAD`, but the no-op
/// build still models it as `int`.
pub type ZSTD_pthread_t = c_int;

/// `#define ZSTD_pthread_mutex_init(a, b) ((void)(a), (void)(b), 0)`
#[inline]
fn ZSTD_pthread_mutex_init(_a: *mut ZSTD_pthread_mutex_t, _b: *const c_void) -> c_int {
    0
}
/// `#define ZSTD_pthread_mutex_destroy(a) ((void)(a))`
#[inline]
fn ZSTD_pthread_mutex_destroy(_a: *mut ZSTD_pthread_mutex_t) {}
/// `#define ZSTD_pthread_mutex_lock(a) ((void)(a))`
#[inline]
fn ZSTD_pthread_mutex_lock(_a: *mut ZSTD_pthread_mutex_t) {}
/// `#define ZSTD_pthread_mutex_unlock(a) ((void)(a))`
#[inline]
fn ZSTD_pthread_mutex_unlock(_a: *mut ZSTD_pthread_mutex_t) {}
/// `#define ZSTD_pthread_cond_init(a, b) ((void)(a), (void)(b), 0)`
#[inline]
fn ZSTD_pthread_cond_init(_a: *mut ZSTD_pthread_cond_t, _b: *const c_void) -> c_int {
    0
}
/// `#define ZSTD_pthread_cond_destroy(a) ((void)(a))`
#[inline]
fn ZSTD_pthread_cond_destroy(_a: *mut ZSTD_pthread_cond_t) {}
/// `#define ZSTD_pthread_cond_wait(a, b) ((void)(a), (void)(b))`
#[inline]
fn ZSTD_pthread_cond_wait(_a: *mut ZSTD_pthread_cond_t, _b: *mut ZSTD_pthread_mutex_t) {}
/// `#define ZSTD_pthread_cond_signal(a) ((void)(a))`
#[inline]
fn ZSTD_pthread_cond_signal(_a: *mut ZSTD_pthread_cond_t) {}
/// `#define ZSTD_pthread_cond_broadcast(a) ((void)(a))`
#[inline]
fn ZSTD_pthread_cond_broadcast(_a: *mut ZSTD_pthread_cond_t) {}

/*-*************************************
*  `cover.h` types
***************************************/

/// `COVER_best_t`
#[repr(C)]
pub struct COVER_best_t {
    pub mutex: ZSTD_pthread_mutex_t,
    pub cond: ZSTD_pthread_cond_t,
    pub liveJobs: usize,
    pub dict: *mut c_void,
    pub dictSize: usize,
    pub parameters: ZDICT_cover_params_t,
    pub compressedSize: usize,
}

/// `COVER_segment_t`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct COVER_segment_t {
    pub begin: U32,
    pub end: U32,
    pub score: U32,
}

/// `COVER_epoch_info_t`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct COVER_epoch_info_t {
    pub num: U32,
    pub size: U32,
}

/// `COVER_dictSelection_t`
#[repr(C)]
#[derive(Copy, Clone)]
pub struct COVER_dictSelection_t {
    pub dictContent: *mut BYTE,
    pub dictSize: usize,
    pub totalCompressedSize: usize,
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
    fn ZDICT_isError(errorCode: usize) -> c_uint;
    fn ZSTD_compressBound(srcSize: usize) -> usize;
    /* NOTE: declared with the same pointee types as the definitions in
     * compress/zstd_compress.rs so that rustc does not report
     * `clashing_extern_declarations`; ABI-wise these are all plain pointers. */
    fn ZSTD_createCCtx() -> *mut crate::compress::zstd_compress_internal::ZSTD_CCtx;
    fn ZSTD_freeCCtx(cctx: *mut crate::compress::zstd_compress_internal::ZSTD_CCtx) -> usize;
    fn ZSTD_createCDict(
        dictBuffer: *const c_void,
        dictSize: usize,
        compressionLevel: c_int,
    ) -> *mut crate::compress::zstd_compress_internal::ZSTD_CDict;
    fn ZSTD_freeCDict(cdict: *mut crate::compress::zstd_compress_internal::ZSTD_CDict) -> usize;
    fn ZSTD_compress_usingCDict(
        cctx: *mut c_void,
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
        cdict: *const c_void,
    ) -> usize;
}

/*-*************************************
*  Constants
***************************************/

/// `#define COVER_MAX_SAMPLES_SIZE (sizeof(size_t) == 8 ? ((unsigned)-1) : ((unsigned)1 GB))`
const COVER_MAX_SAMPLES_SIZE: c_uint = c_uint::MAX;
/// `#define COVER_DEFAULT_SPLITPOINT 1.0`
const COVER_DEFAULT_SPLITPOINT: f64 = 1.0;

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

/// `LOCALDISPLAYUPDATE()` — the `DISPLAY()` body is a no-op here, but the
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
* Hash table
***************************************/

/// `#define MAP_EMPTY_VALUE ((U32)-1)`
const MAP_EMPTY_VALUE: U32 = U32::MAX;

/// `COVER_map_pair_t`
#[repr(C)]
#[derive(Copy, Clone)]
struct COVER_map_pair_t {
    key: U32,
    value: U32,
}

/// `COVER_map_t`
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
        (*map).size as usize * size_of::<COVER_map_pair_t>(),
    );
}

/// Initializes a map of the given size.
/// Returns 1 on success and 0 on failure.
unsafe fn COVER_map_init(map: *mut COVER_map_t, size: U32) -> c_int {
    (*map).sizeLog = ZSTD_highbit32(size) + 2;
    (*map).size = (1u32).wrapping_shl((*map).sizeLog);
    (*map).sizeMask = (*map).size.wrapping_sub(1);
    (*map).data =
        malloc((*map).size as usize * size_of::<COVER_map_pair_t>()) as *mut COVER_map_pair_t;
    if (*map).data.is_null() {
        (*map).sizeLog = 0;
        (*map).size = 0;
        return 0;
    }
    COVER_map_clear(map);
    1
}

/// `static const U32 COVER_prime4bytes = 2654435761U;`
const COVER_prime4bytes: U32 = 2654435761u32;

/// Internal hash function
unsafe fn COVER_map_hash(map: *mut COVER_map_t, key: U32) -> U32 {
    key.wrapping_mul(COVER_prime4bytes)
        .wrapping_shr(32u32.wrapping_sub((*map).sizeLog))
}

/// Helper function that returns the index that a key should be placed into.
unsafe fn COVER_map_index(map: *mut COVER_map_t, key: U32) -> U32 {
    let hash: U32 = COVER_map_hash(map, key);
    let mut i: U32 = hash;
    loop {
        let pos: *mut COVER_map_pair_t = (*map).data.add(i as usize);
        if (*pos).value == MAP_EMPTY_VALUE {
            return i;
        }
        if (*pos).key == key {
            return i;
        }
        i = i.wrapping_add(1) & (*map).sizeMask;
    }
}

/// Returns the pointer to the value for key.
/// If key is not in the map, it is inserted and the value is set to 0.
unsafe fn COVER_map_at(map: *mut COVER_map_t, key: U32) -> *mut U32 {
    let pos: *mut COVER_map_pair_t = (*map).data.add(COVER_map_index(map, key) as usize);
    if (*pos).value == MAP_EMPTY_VALUE {
        (*pos).key = key;
        (*pos).value = 0;
    }
    ptr::addr_of_mut!((*pos).value)
}

/// Deletes key from the map if present.
unsafe fn COVER_map_remove(map: *mut COVER_map_t, key: U32) {
    let mut i: U32 = COVER_map_index(map, key);
    let mut del: *mut COVER_map_pair_t = (*map).data.add(i as usize);
    let mut shift: U32 = 1;
    if (*del).value == MAP_EMPTY_VALUE {
        return;
    }
    i = i.wrapping_add(1) & (*map).sizeMask;
    loop {
        let pos: *mut COVER_map_pair_t = (*map).data.add(i as usize);
        /* If the position is empty we are done */
        if (*pos).value == MAP_EMPTY_VALUE {
            (*del).value = MAP_EMPTY_VALUE;
            return;
        }
        /* If pos can be moved to del do so */
        if (i.wrapping_sub(COVER_map_hash(map, (*pos).key)) & (*map).sizeMask) >= shift {
            (*del).key = (*pos).key;
            (*del).value = (*pos).value;
            del = pos;
            shift = 1;
        } else {
            shift = shift.wrapping_add(1);
        }
        i = i.wrapping_add(1) & (*map).sizeMask;
    }
}

/// Destroys a map that is inited with `COVER_map_init()`.
unsafe fn COVER_map_destroy(map: *mut COVER_map_t) {
    if !(*map).data.is_null() {
        free((*map).data as *mut c_void);
    }
    (*map).data = ptr::null_mut();
    (*map).size = 0;
}

/*-*************************************
* Context
***************************************/

/// `COVER_ctx_t`
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

/* `_GNU_SOURCE` is defined on linux, so the `g_coverCtx` C90 fallback global is
 * not compiled. */

/*-*************************************
*  Helper functions
***************************************/

/// Returns the sum of the sample sizes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_sum(samplesSizes: *const usize, nbSamples: c_uint) -> usize {
    let mut sum: usize = 0;
    let mut i: c_uint = 0;
    while i < nbSamples {
        sum = sum.wrapping_add(*samplesSizes.add(i as usize));
        i = i.wrapping_add(1);
    }
    sum
}

/// Returns -1 if the dmer at lp is less than the dmer at rp.
/// Return 0 if the dmers at lp and rp are equal.
/// Returns 1 if the dmer at lp is greater than the dmer at rp.
unsafe fn COVER_cmp(ctx: *mut COVER_ctx_t, lp: *const c_void, rp: *const c_void) -> c_int {
    let lhs: U32 = *(lp as *const U32);
    let rhs: U32 = *(rp as *const U32);
    memcmp(
        (*ctx).samples.add(lhs as usize) as *const c_void,
        (*ctx).samples.add(rhs as usize) as *const c_void,
        (*ctx).d as usize,
    )
}

/// Faster version for d <= 8.
unsafe fn COVER_cmp8(ctx: *mut COVER_ctx_t, lp: *const c_void, rp: *const c_void) -> c_int {
    let mask: U64 = if (*ctx).d == 8 {
        U64::MAX
    } else {
        (1u64).wrapping_shl(8u32.wrapping_mul((*ctx).d)).wrapping_sub(1)
    };
    let lhs: U64 = MEM_readLE64(
        (*ctx).samples.add(*(lp as *const U32) as usize) as *const c_void,
    ) & mask;
    let rhs: U64 = MEM_readLE64(
        (*ctx).samples.add(*(rp as *const U32) as usize) as *const c_void,
    ) & mask;
    if lhs < rhs {
        return -1;
    }
    (lhs > rhs) as c_int
}

/// Same as `COVER_cmp()` except ties are broken by pointer value.
/// glibc `qsort_r()` comparator signature.
unsafe extern "C" fn COVER_strict_cmp(
    lp: *const c_void,
    rp: *const c_void,
    g_coverCtx: *mut c_void,
) -> c_int {
    let mut result: c_int = COVER_cmp(g_coverCtx as *mut COVER_ctx_t, lp, rp);
    if result == 0 {
        result = if lp < rp { -1 } else { 1 };
    }
    result
}

/// Faster version for d <= 8.
unsafe extern "C" fn COVER_strict_cmp8(
    lp: *const c_void,
    rp: *const c_void,
    g_coverCtx: *mut c_void,
) -> c_int {
    let mut result: c_int = COVER_cmp8(g_coverCtx as *mut COVER_ctx_t, lp, rp);
    if result == 0 {
        result = if lp < rp { -1 } else { 1 };
    }
    result
}

/// Abstract away divergence of `qsort_r()` parameters — the `_GNU_SOURCE`
/// variant.
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
        size_of::<U32>(),
        Some(cmp),
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
    let mut count: usize = last.offset_from(first) as usize;
    while count != 0 {
        let step: usize = count / 2;
        let mut ptr: *const usize = first;
        ptr = ptr.add(step);
        if *ptr < value {
            ptr = ptr.add(1);
            first = ptr;
            count = count.wrapping_sub(step + 1);
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
    let mut ptr: *const BYTE = data as *const BYTE;
    let mut num: usize = 0;
    while num < count {
        let mut grpEnd: *const BYTE = ptr.add(size);
        num = num.wrapping_add(1);
        while num < count && cmp(ctx, ptr as *const c_void, grpEnd as *const c_void) == 0 {
            grpEnd = grpEnd.add(size);
            num = num.wrapping_add(1);
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
    let mut grpPtr: *const U32 = group as *const U32;
    let grpEnd: *const U32 = groupEnd as *const U32;
    /* The dmerId is how we will reference this dmer. */
    let dmerId: U32 = grpPtr.offset_from((*ctx).suffix as *const U32) as U32;
    /* Count the number of samples this dmer shows up in */
    let mut freq: U32 = 0;
    /* Details */
    let mut curOffsetPtr: *const usize = (*ctx).offsets as *const usize;
    let offsetsEnd: *const usize = ((*ctx).offsets as *const usize).add((*ctx).nbSamples);
    /* Once *grpPtr >= curSampleEnd this occurrence of the dmer is in a
     * different sample than the last. */
    let mut curSampleEnd: usize = *(*ctx).offsets.add(0);
    while grpPtr != grpEnd {
        /* Save the dmerId for this position so we can get back to it. */
        *(*ctx).dmerAt.add(*grpPtr as usize) = dmerId;
        /* Dictionaries only help for the first reference to the dmer. */
        if (*grpPtr as usize) < curSampleEnd {
            grpPtr = grpPtr.add(1);
            continue;
        }
        freq = freq.wrapping_add(1);
        /* Binary search to find the end of the sample *grpPtr is in. */
        if grpPtr.add(1) != grpEnd {
            let sampleEndPtr: *const usize =
                COVER_lower_bound(curOffsetPtr, offsetsEnd, *grpPtr as usize);
            curSampleEnd = *sampleEndPtr;
            curOffsetPtr = sampleEndPtr.add(1);
        }
        grpPtr = grpPtr.add(1);
    }
    /* We store the frequency of the dmer in the first position of the group,
     * which is dmerId. */
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
    let k: U32 = parameters.k;
    let d: U32 = parameters.d;
    let dmersInK: U32 = k.wrapping_sub(d).wrapping_add(1);
    /* Try each segment (activeSegment) and save the best (bestSegment) */
    let mut bestSegment: COVER_segment_t = COVER_segment_t {
        begin: 0,
        end: 0,
        score: 0,
    };
    let mut activeSegment: COVER_segment_t;
    /* Reset the activeDmers in the segment */
    COVER_map_clear(activeDmers);
    /* The activeSegment starts at the beginning of the epoch. */
    activeSegment = COVER_segment_t {
        begin: begin,
        end: begin,
        score: 0,
    };
    /* Slide the activeSegment through the whole epoch.
     * Save the best segment in bestSegment. */
    while activeSegment.end < end {
        /* The dmerId for the dmer at the next position */
        let newDmer: U32 = *(*ctx).dmerAt.add(activeSegment.end as usize);
        /* The entry in activeDmers for this dmerId */
        let newDmerOcc: *mut U32 = COVER_map_at(activeDmers, newDmer);
        /* If the dmer isn't already present in the segment add its score. */
        if *newDmerOcc == 0 {
            activeSegment.score = activeSegment
                .score
                .wrapping_add(*freqs.add(newDmer as usize));
        }
        /* Add the dmer to the segment */
        activeSegment.end = activeSegment.end.wrapping_add(1);
        *newDmerOcc = (*newDmerOcc).wrapping_add(1);

        /* If the window is now too large, drop the first position */
        if activeSegment.end.wrapping_sub(activeSegment.begin) == dmersInK.wrapping_add(1) {
            let delDmer: U32 = *(*ctx).dmerAt.add(activeSegment.begin as usize);
            let delDmerOcc: *mut U32 = COVER_map_at(activeDmers, delDmer);
            activeSegment.begin = activeSegment.begin.wrapping_add(1);
            *delDmerOcc = (*delDmerOcc).wrapping_sub(1);
            /* If this is the last occurrence of the dmer, subtract its score */
            if *delDmerOcc == 0 {
                COVER_map_remove(activeDmers, delDmer);
                activeSegment.score = activeSegment
                    .score
                    .wrapping_sub(*freqs.add(delDmer as usize));
            }
        }

        /* If this segment is the best so far save it */
        if activeSegment.score > bestSegment.score {
            bestSegment = activeSegment;
        }
    }
    {
        /* Trim off the zero frequency head and tail from the segment. */
        let mut newBegin: U32 = bestSegment.end;
        let mut newEnd: U32 = bestSegment.begin;
        let mut pos: U32 = bestSegment.begin;
        while pos != bestSegment.end {
            let freq: U32 = *freqs.add(*(*ctx).dmerAt.add(pos as usize) as usize);
            if freq != 0 {
                newBegin = MIN(newBegin, pos);
                newEnd = pos.wrapping_add(1);
            }
            pos = pos.wrapping_add(1);
        }
        bestSegment.begin = newBegin;
        bestSegment.end = newEnd;
    }
    {
        /* Zero out the frequency of each dmer covered by the chosen segment. */
        let mut pos: U32 = bestSegment.begin;
        while pos != bestSegment.end {
            *freqs.add(*(*ctx).dmerAt.add(pos as usize) as usize) = 0;
            pos = pos.wrapping_add(1);
        }
    }
    bestSegment
}

/// Check the validity of the parameters.
/// Returns non-zero if the parameters are valid and 0 otherwise.
unsafe fn COVER_checkParameters(
    parameters: ZDICT_cover_params_t,
    maxDictSize: usize,
) -> c_int {
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
        (*ctx).suffix = ptr::null_mut();
    }
    if !(*ctx).freqs.is_null() {
        free((*ctx).freqs as *mut c_void);
        (*ctx).freqs = ptr::null_mut();
    }
    if !(*ctx).dmerAt.is_null() {
        free((*ctx).dmerAt as *mut c_void);
        (*ctx).dmerAt = ptr::null_mut();
    }
    if !(*ctx).offsets.is_null() {
        free((*ctx).offsets as *mut c_void);
        (*ctx).offsets = ptr::null_mut();
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
        || totalSamplesSize >= COVER_MAX_SAMPLES_SIZE as usize
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
    memset(ctx as *mut c_void, 0, size_of::<COVER_ctx_t>());
    (*ctx).samples = samples;
    (*ctx).samplesSizes = samplesSizes;
    (*ctx).nbSamples = nbSamples as usize;
    (*ctx).nbTrainSamples = nbTrainSamples as usize;
    (*ctx).nbTestSamples = nbTestSamples as usize;
    /* Partial suffix array */
    (*ctx).suffixSize = trainingSamplesSize
        .wrapping_sub(MAX(d as usize, size_of::<U64>()))
        .wrapping_add(1);
    (*ctx).suffix = malloc((*ctx).suffixSize.wrapping_mul(size_of::<U32>())) as *mut U32;
    /* Maps index to the dmerID */
    (*ctx).dmerAt = malloc((*ctx).suffixSize.wrapping_mul(size_of::<U32>())) as *mut U32;
    /* The offsets of each file */
    (*ctx).offsets =
        malloc((nbSamples as usize + 1).wrapping_mul(size_of::<usize>())) as *mut usize;
    if (*ctx).suffix.is_null() || (*ctx).dmerAt.is_null() || (*ctx).offsets.is_null() {
        COVER_ctx_destroy(ctx);
        return ERROR(ZSTD_error_memory_allocation);
    }
    (*ctx).freqs = ptr::null_mut();
    (*ctx).d = d;

    /* Fill offsets from the samplesSizes */
    {
        let mut i: U32;
        *(*ctx).offsets.add(0) = 0;
        i = 1;
        while i <= nbSamples {
            *(*ctx).offsets.add(i as usize) = (*(*ctx).offsets.add(i as usize - 1))
                .wrapping_add(*samplesSizes.add(i as usize - 1));
            i = i.wrapping_add(1);
        }
    }
    {
        /* suffix is a partial suffix array.
         * It only sorts suffixes by their first parameters.d bytes.
         * The sort is stable, so each dmer group is sorted by position in input.
         */
        let mut i: U32 = 0;
        while (i as usize) < (*ctx).suffixSize {
            *(*ctx).suffix.add(i as usize) = i;
            i = i.wrapping_add(1);
        }
        stableSort(ctx);
    }
    /* For each dmer group (group of positions with the same first d bytes):
     * 1. For each position we set dmerAt[position] = dmerID.
     * 2. We calculate how many samples the dmer occurs in and save it in
     *    freqs[dmerId].
     */
    {
        let cmp: unsafe fn(*mut COVER_ctx_t, *const c_void, *const c_void) -> c_int =
            if (*ctx).d <= 8 { COVER_cmp8 } else { COVER_cmp };
        COVER_groupBy(
            (*ctx).suffix as *const c_void,
            (*ctx).suffixSize,
            size_of::<U32>(),
            ctx,
            cmp,
            COVER_group,
        );
    }
    (*ctx).freqs = (*ctx).suffix;
    (*ctx).suffix = ptr::null_mut();
    0
}

/// Warns the user when their corpus is too small.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_warnOnSmallCorpus(
    maxDictSize: usize,
    nbDmers: usize,
    displayLevel: c_int,
) {
    let ratio: f64 = nbDmers as f64 / maxDictSize as f64;
    if ratio >= 10.0 {
        return;
    }
    let _ = displayLevel;
    /* LOCALDISPLAYLEVEL(displayLevel, 1, "WARNING: ...") — display only. */
}

/// Computes the number of epochs and the size of each epoch.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_computeEpochs(
    maxDictSize: U32,
    nbDmers: U32,
    k: U32,
    passes: U32,
) -> COVER_epoch_info_t {
    let minEpochSize: U32 = k.wrapping_mul(10);
    let mut epochs: COVER_epoch_info_t = COVER_epoch_info_t { num: 0, size: 0 };
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
    dictBufferCapacity: usize,
    parameters: ZDICT_cover_params_t,
) -> usize {
    let dict: *mut BYTE = dictBuffer as *mut BYTE;
    let mut tail: usize = dictBufferCapacity;
    /* Divide the data into epochs. We will select one segment from each epoch. */
    let epochs: COVER_epoch_info_t = COVER_computeEpochs(
        dictBufferCapacity as U32,
        (*ctx).suffixSize as U32,
        parameters.k,
        4,
    );
    let maxZeroScoreRun: usize = MAX(10u32, MIN(100u32, epochs.num >> 3)) as usize;
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
            let segment: COVER_segment_t = COVER_selectSegment(
                ctx,
                freqs,
                activeDmers,
                epochBegin,
                epochEnd,
                parameters,
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

/// `ZDICT_trainFromBuffer_cover()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_trainFromBuffer_cover(
    dictBuffer: *mut c_void,
    dictBufferCapacity: usize,
    samplesBuffer: *const c_void,
    samplesSizes: *const usize,
    nbSamples: c_uint,
    mut parameters: ZDICT_cover_params_t,
) -> usize {
    let dict: *mut BYTE = dictBuffer as *mut BYTE;
    let mut ctx: COVER_ctx_t = core::mem::zeroed();
    let mut activeDmers: COVER_map_t = core::mem::zeroed();
    parameters.splitPoint = 1.0;
    /* Initialize global data */
    g_displayLevel = parameters.zParams.notificationLevel as c_int;
    /* Checks */
    if COVER_checkParameters(parameters, dictBufferCapacity) == 0 {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if nbSamples == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    /* Initialize context and activeDmers */
    {
        let initVal: usize = COVER_ctx_init(
            ptr::addr_of_mut!(ctx),
            samplesBuffer,
            samplesSizes,
            nbSamples,
            parameters.d,
            parameters.splitPoint,
        );
        if ERR_isError(initVal) != 0 {
            return initVal;
        }
    }
    COVER_warnOnSmallCorpus(dictBufferCapacity, ctx.suffixSize, g_displayLevel);
    if COVER_map_init(
        ptr::addr_of_mut!(activeDmers),
        parameters.k.wrapping_sub(parameters.d).wrapping_add(1),
    ) == 0
    {
        COVER_ctx_destroy(ptr::addr_of_mut!(ctx));
        return ERROR(ZSTD_error_memory_allocation);
    }

    {
        let tail: usize = COVER_buildDictionary(
            ptr::addr_of!(ctx),
            ctx.freqs,
            ptr::addr_of_mut!(activeDmers),
            dictBuffer,
            dictBufferCapacity,
            parameters,
        );
        let dictionarySize: usize = ZDICT_finalizeDictionary(
            dict as *mut c_void,
            dictBufferCapacity,
            dict.add(tail) as *const c_void,
            dictBufferCapacity.wrapping_sub(tail),
            samplesBuffer,
            samplesSizes,
            nbSamples,
            parameters.zParams,
        );
        COVER_ctx_destroy(ptr::addr_of_mut!(ctx));
        COVER_map_destroy(ptr::addr_of_mut!(activeDmers));
        dictionarySize
    }
}

/// Checks total compressed size of a dictionary.
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
    let mut totalCompressedSize: usize = ERROR(ZSTD_error_GENERIC);
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
            maxSampleSize = MAX(*samplesSizes.add(i), maxSampleSize);
            i = i.wrapping_add(1);
        }
        dstCapacity = ZSTD_compressBound(maxSampleSize);
        dst = malloc(dstCapacity);
    }
    /* Create the cctx and cdict */
    cctx = ZSTD_createCCtx() as *mut c_void;
    cdict = ZSTD_createCDict(
        dict as *const c_void,
        dictBufferCapacity,
        parameters.zParams.compressionLevel,
    ) as *mut c_void;
    '_compressCleanup: {
        if dst.is_null() || cctx.is_null() || cdict.is_null() {
            break '_compressCleanup;
        }
        /* Compress each sample and sum their sizes (or error) */
        totalCompressedSize = dictBufferCapacity;
        i = if parameters.splitPoint < 1.0 {
            nbTrainSamples
        } else {
            0
        };
        while i < nbSamples {
            let size: usize = ZSTD_compress_usingCDict(
                cctx,
                dst,
                dstCapacity,
                samples.add(*offsets.add(i)) as *const c_void,
                *samplesSizes.add(i),
                cdict as *const c_void,
            );
            if ERR_isError(size) != 0 {
                totalCompressedSize = size;
                break '_compressCleanup;
            }
            totalCompressedSize = totalCompressedSize.wrapping_add(size);
            i = i.wrapping_add(1);
        }
    }
    ZSTD_freeCCtx(cctx as *mut crate::compress::zstd_compress_internal::ZSTD_CCtx);
    ZSTD_freeCDict(cdict as *mut crate::compress::zstd_compress_internal::ZSTD_CDict);
    if !dst.is_null() {
        free(dst);
    }
    totalCompressedSize
}

/// Initialize the `COVER_best_t`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_init(best: *mut COVER_best_t) {
    if best.is_null() {
        return; /* compatible with init on NULL */
    }
    let _ = ZSTD_pthread_mutex_init(ptr::addr_of_mut!((*best).mutex), ptr::null());
    let _ = ZSTD_pthread_cond_init(ptr::addr_of_mut!((*best).cond), ptr::null());
    (*best).liveJobs = 0;
    (*best).dict = ptr::null_mut();
    (*best).dictSize = 0;
    (*best).compressedSize = !0usize;
    memset(
        ptr::addr_of_mut!((*best).parameters) as *mut c_void,
        0,
        size_of::<ZDICT_cover_params_t>(),
    );
}

/// Wait until `liveJobs == 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_wait(best: *mut COVER_best_t) {
    if best.is_null() {
        return;
    }
    ZSTD_pthread_mutex_lock(ptr::addr_of_mut!((*best).mutex));
    while (*best).liveJobs != 0 {
        ZSTD_pthread_cond_wait(
            ptr::addr_of_mut!((*best).cond),
            ptr::addr_of_mut!((*best).mutex),
        );
    }
    ZSTD_pthread_mutex_unlock(ptr::addr_of_mut!((*best).mutex));
}

/// Call `COVER_best_wait()` and then destroy the `COVER_best_t`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_destroy(best: *mut COVER_best_t) {
    if best.is_null() {
        return;
    }
    COVER_best_wait(best);
    if !(*best).dict.is_null() {
        free((*best).dict);
    }
    ZSTD_pthread_mutex_destroy(ptr::addr_of_mut!((*best).mutex));
    ZSTD_pthread_cond_destroy(ptr::addr_of_mut!((*best).cond));
}

/// Called when a thread is about to be launched.  Increments `liveJobs`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_start(best: *mut COVER_best_t) {
    if best.is_null() {
        return;
    }
    ZSTD_pthread_mutex_lock(ptr::addr_of_mut!((*best).mutex));
    (*best).liveJobs = (*best).liveJobs.wrapping_add(1);
    ZSTD_pthread_mutex_unlock(ptr::addr_of_mut!((*best).mutex));
}

/// Called when a thread finishes executing, both on error or success.
/// Decrements `liveJobs` and signals any waiting threads if `liveJobs == 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_finish(
    best: *mut COVER_best_t,
    parameters: ZDICT_cover_params_t,
    selection: COVER_dictSelection_t,
) {
    let dict: *mut c_void = selection.dictContent as *mut c_void;
    let compressedSize: usize = selection.totalCompressedSize;
    let dictSize: usize = selection.dictSize;
    if best.is_null() {
        return;
    }
    {
        let liveJobs: usize;
        ZSTD_pthread_mutex_lock(ptr::addr_of_mut!((*best).mutex));
        (*best).liveJobs = (*best).liveJobs.wrapping_sub(1);
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
                    (*best).compressedSize = ERROR(ZSTD_error_GENERIC);
                    (*best).dictSize = 0;
                    ZSTD_pthread_cond_signal(ptr::addr_of_mut!((*best).cond));
                    ZSTD_pthread_mutex_unlock(ptr::addr_of_mut!((*best).mutex));
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
            ZSTD_pthread_cond_broadcast(ptr::addr_of_mut!((*best).cond));
        }
        ZSTD_pthread_mutex_unlock(ptr::addr_of_mut!((*best).mutex));
    }
}

/// `static COVER_dictSelection_t setDictSelection(BYTE* buf, size_t s, size_t csz)`
unsafe fn setDictSelection(buf: *mut BYTE, s: usize, csz: usize) -> COVER_dictSelection_t {
    COVER_dictSelection_t {
        dictContent: buf,
        dictSize: s,
        totalCompressedSize: csz,
    }
}

/// Returns a struct where `return.totalCompressedSize` is a ZSTD error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_dictSelectionError(error: usize) -> COVER_dictSelection_t {
    setDictSelection(ptr::null_mut(), 0, error)
}

/// Error function for `COVER_selectDict()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_dictSelectionIsError(
    selection: COVER_dictSelection_t,
) -> c_uint {
    (ERR_isError(selection.totalCompressedSize) != 0 || selection.dictContent.is_null()) as c_uint
}

/// Frees up used memory from a newly created dictionary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_dictSelectionFree(selection: COVER_dictSelection_t) {
    free(selection.dictContent as *mut c_void);
}

/// Called to finalize the dictionary and select one based on whether or not
/// the shrink-dict flag was enabled.
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
    let customDictContentEnd: *mut BYTE = customDictContent.add(dictContentSize);

    let largestDictbuffer: *mut BYTE = malloc(dictBufferCapacity) as *mut BYTE;
    let candidateDictBuffer: *mut BYTE = malloc(dictBufferCapacity) as *mut BYTE;
    let regressionTolerance: f64 = (params.shrinkDictMaxRegression as f64 / 100.0) + 1.00;

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

    if ERR_isError(totalCompressedSize) != 0 {
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

        if ERR_isError(totalCompressedSize) != 0 {
            free(largestDictbuffer as *mut c_void);
            free(candidateDictBuffer as *mut c_void);
            return COVER_dictSelectionError(totalCompressedSize);
        }

        if (totalCompressedSize as f64) <= (largestCompressed as f64) * regressionTolerance {
            free(largestDictbuffer as *mut c_void);
            return setDictSelection(candidateDictBuffer, dictContentSize, totalCompressedSize);
        }
        dictContentSize = dictContentSize.wrapping_mul(2);
    }
    dictContentSize = largestDict;
    totalCompressedSize = largestCompressed;
    free(candidateDictBuffer as *mut c_void);
    setDictSelection(largestDictbuffer, dictContentSize, totalCompressedSize)
}

/// Parameters for `COVER_tryParameters()`.
#[repr(C)]
struct COVER_tryParameters_data_t {
    ctx: *const COVER_ctx_t,
    best: *mut COVER_best_t,
    dictBufferCapacity: usize,
    parameters: ZDICT_cover_params_t,
}

/// Tries a set of parameters and updates the `COVER_best_t` with the results.
/// It takes its parameters as an *OWNING* opaque pointer to support threading.
unsafe extern "C" fn COVER_tryParameters(opaque: *mut c_void) {
    /* Save parameters as local variables */
    let data: *mut COVER_tryParameters_data_t = opaque as *mut COVER_tryParameters_data_t;
    let ctx: *const COVER_ctx_t = (*data).ctx;
    let parameters: ZDICT_cover_params_t = (*data).parameters;
    let dictBufferCapacity: usize = (*data).dictBufferCapacity;
    let totalCompressedSize: usize = ERROR(ZSTD_error_GENERIC);
    /* Allocate space for hash table, dict, and freqs */
    let mut activeDmers: COVER_map_t = core::mem::zeroed();
    let dict: *mut BYTE = malloc(dictBufferCapacity) as *mut BYTE;
    let mut selection: COVER_dictSelection_t =
        COVER_dictSelectionError(ERROR(ZSTD_error_GENERIC));
    let freqs: *mut U32 =
        malloc((*ctx).suffixSize.wrapping_mul(size_of::<U32>())) as *mut U32;
    '_cleanup: {
        if COVER_map_init(
            ptr::addr_of_mut!(activeDmers),
            parameters.k.wrapping_sub(parameters.d).wrapping_add(1),
        ) == 0
        {
            break '_cleanup;
        }
        if dict.is_null() || freqs.is_null() {
            break '_cleanup;
        }
        /* Copy the frequencies because we need to modify them */
        memcpy(
            freqs as *mut c_void,
            (*ctx).freqs as *const c_void,
            (*ctx).suffixSize.wrapping_mul(size_of::<U32>()),
        );
        /* Build the dictionary */
        {
            let tail: usize = COVER_buildDictionary(
                ctx,
                freqs,
                ptr::addr_of_mut!(activeDmers),
                dict as *mut c_void,
                dictBufferCapacity,
                parameters,
            );
            selection = COVER_selectDict(
                dict.add(tail),
                dictBufferCapacity,
                dictBufferCapacity.wrapping_sub(tail),
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
                break '_cleanup;
            }
        }
    }
    free(dict as *mut c_void);
    COVER_best_finish((*data).best, parameters, selection);
    free(data as *mut c_void);
    COVER_map_destroy(ptr::addr_of_mut!(activeDmers));
    COVER_dictSelectionFree(selection);
    free(freqs as *mut c_void);
}

/// `ZDICT_optimizeTrainFromBuffer_cover()`
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
    let nbThreads: c_uint = (*parameters).nbThreads;
    let splitPoint: f64 = if (*parameters).splitPoint <= 0.0 {
        COVER_DEFAULT_SPLITPOINT
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
        let mut ctx: COVER_ctx_t = core::mem::zeroed();
        {
            let initVal: usize = COVER_ctx_init(
                ptr::addr_of_mut!(ctx),
                samplesBuffer,
                samplesSizes,
                nbSamples,
                d,
                splitPoint,
            );
            if ERR_isError(initVal) != 0 {
                COVER_best_destroy(ptr::addr_of_mut!(best));
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
            'k_body: {
                /* Prepare the arguments */
                let data: *mut COVER_tryParameters_data_t =
                    malloc(size_of::<COVER_tryParameters_data_t>())
                        as *mut COVER_tryParameters_data_t;
                if data.is_null() {
                    COVER_best_destroy(ptr::addr_of_mut!(best));
                    COVER_ctx_destroy(ptr::addr_of_mut!(ctx));
                    POOL_free(pool);
                    return ERROR(ZSTD_error_memory_allocation);
                }
                (*data).ctx = ptr::addr_of!(ctx);
                (*data).best = ptr::addr_of_mut!(best);
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
                    break 'k_body;
                }
                /* Call the function and pass ownership of data to it */
                COVER_best_start(ptr::addr_of_mut!(best));
                if !pool.is_null() {
                    POOL_add(pool, Some(COVER_tryParameters), data as *mut c_void);
                } else {
                    COVER_tryParameters(data as *mut c_void);
                }
                /* Print status */
                LOCALDISPLAYUPDATE(displayLevel, 2);
                iteration = iteration.wrapping_add(1);
            }
            k = k.wrapping_add(kStepSize);
        }
        COVER_best_wait(ptr::addr_of_mut!(best));
        COVER_ctx_destroy(ptr::addr_of_mut!(ctx));
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
        *parameters = best.parameters;
        memcpy(dictBuffer, best.dict as *const c_void, dictSize);
        COVER_best_destroy(ptr::addr_of_mut!(best));
        POOL_free(pool);
        dictSize
    }
}
