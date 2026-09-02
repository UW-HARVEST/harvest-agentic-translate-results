//!
//! Literal, semantics-preserving transliteration of `zstdmt_compress.c` +
//! `zstdmt_compress.h`.
//!
//! Build configuration: `DYNAMIC_BMI2=0`, **no `ZSTD_MULTITHREAD`**,
//! `DEBUGLEVEL 0` (asserts / DEBUGLOG / DEBUG_PRINTHEX dropped), `ZSTD_TRACE==1`.
//!
//! Consequences of `ZSTD_MULTITHREAD` being undefined (see `common/threading.h`
//! and the already-translated `common/pool.rs`):
//!   * `ZSTD_pthread_mutex_t` / `ZSTD_pthread_cond_t` are trivial `int`
//!     placeholders and every `ZSTD_pthread_*` op is a no-op.
//!   * `POOL_add` / `POOL_tryAdd` execute the job SYNCHRONOUSLY.
//!   * `ZSTDMT_createCCtx_advanced` `(void)`s its args and returns NULL.
//!
//! Threading placeholder types are modelled here as `c_int` fields (matching the
//! non-MT `typedef int` in threading.h) and every pthread op is an inline no-op.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
#![allow(unused_variables)]

use core::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};
use core::ptr::null_mut;

use crate::common::error_private::*;
use crate::common::mem::*;
use crate::common::pool::{POOL_add, POOL_ctx, POOL_free, POOL_sizeof, POOL_tryAdd, POOL_create_advanced, POOL_resize};
use crate::common::xxhash::{XXH64_state_t, ZSTD_XXH64_digest, ZSTD_XXH64_reset, ZSTD_XXH64_update};
use crate::common::zstd_common::ZSTD_isError;
use crate::common::zstd_h::*;
use crate::common::zstd_internal::{
    MAX, MIN, ZSTD_customCalloc, ZSTD_customFree, ZSTD_customMalloc, ZSTD_customMem,
};

use crate::common::bits::ZSTD_highbit32;
use crate::compress::zstd_compress_internal::*;

/* ZSTD_threadPool == POOL_ctx (see zstd_h.rs). */
use crate::common::zstd_h::ZSTD_threadPool;

/* ===   Functions provided by other cdylib translation units   ===
 * These are exported symbols of the same shared library, so they link.
 * (Some are already re-exported through `zstd_compress_internal::*` — those we
 *  do not re-declare here to avoid duplicate-symbol errors.)
 */
unsafe extern "C" {
    fn ZSTD_createCCtx_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CCtx;
    fn ZSTD_freeCCtx(cctx: *mut ZSTD_CCtx) -> size_t;
    fn ZSTD_sizeof_CCtx(cctx: *const ZSTD_CCtx) -> size_t;

    fn ZSTD_CCtxParams_setParameter(
        params: *mut ZSTD_CCtx_params,
        param: ZSTD_cParameter,
        value: c_int,
    ) -> size_t;
    fn ZSTD_invalidateRepCodes(cctx: *mut ZSTD_CCtx);

    fn ZSTD_createCDict_advanced(
        dict: *const c_void,
        dictSize: size_t,
        dictLoadMethod: ZSTD_dictLoadMethod_e,
        dictContentType: ZSTD_dictContentType_e,
        cParams: ZSTD_compressionParameters,
        customMem: ZSTD_customMem,
    ) -> *mut ZSTD_CDict;
    fn ZSTD_freeCDict(cdict: *mut ZSTD_CDict) -> size_t;
    fn ZSTD_sizeof_CDict(cdict: *const ZSTD_CDict) -> size_t;

    fn ZSTD_checkCParams(cParams: ZSTD_compressionParameters) -> size_t;
    fn ZSTD_compressBound(srcSize: size_t) -> size_t;

    fn ZSTD_ldm_adjustParameters(params: *mut ldmParams_t, cParams: *const ZSTD_compressionParameters);
    fn ZSTD_ldm_getMaxNbSeq(params: ldmParams_t, maxChunkSize: size_t) -> size_t;
    fn ZSTD_ldm_generateSequences(
        ldmState: *mut ldmState_t,
        sequences: *mut RawSeqStore_t,
        params: *const ldmParams_t,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_ldm_fillHashTable(
        ldmState: *mut ldmState_t,
        ip: *const BYTE,
        iend: *const BYTE,
        params: *const ldmParams_t,
    );
}

/* ======   Constants (zstdmt_compress.h)   ====== */

/* ZSTDMT_NBWORKERS_MAX ((sizeof(void*)==4) ? 64 : 256) */
pub const ZSTDMT_NBWORKERS_MAX: c_uint =
    if core::mem::size_of::<*const c_void>() == 4 { 64 } else { 256 };

/* #define KB *(1 <<10), MB *(1 <<20)  — ZSTDMT_JOBSIZE_MIN (512 KB) */
pub const ZSTDMT_JOBSIZE_MIN: size_t = 512 * (1 << 10);

/* ZSTDMT_JOBLOG_MAX  (MEM_32bits() ? 29 : 30) */
pub fn ZSTDMT_JOBLOG_MAX() -> c_uint {
    if MEM_32bits() != 0 { 29 } else { 30 }
}

/* ZSTDMT_JOBSIZE_MAX (MEM_32bits() ? (512 MB) : (1024 MB)) */
pub fn ZSTDMT_JOBSIZE_MAX() -> size_t {
    if MEM_32bits() != 0 { 512 * (1 << 20) } else { 1024usize * (1 << 20) }
}

/* #define ZSTD_RESIZE_SEQPOOL 0 */

/* =====   Threading placeholders (non-MT build)   =====
 * threading.h (no ZSTD_MULTITHREAD):
 *   typedef int ZSTD_pthread_mutex_t;
 *   typedef int ZSTD_pthread_cond_t;
 * and every op is a no-op. We model the ops as inline no-ops.
 */
pub type ZSTD_pthread_mutex_t = c_int;
pub type ZSTD_pthread_cond_t = c_int;

#[inline(always)]
unsafe fn ZSTD_pthread_mutex_init(_a: *mut ZSTD_pthread_mutex_t, _b: *const c_void) -> c_int {
    0
}
#[inline(always)]
unsafe fn ZSTD_pthread_mutex_destroy(_a: *mut ZSTD_pthread_mutex_t) {}
#[inline(always)]
unsafe fn ZSTD_pthread_mutex_lock(_a: *mut ZSTD_pthread_mutex_t) {}
#[inline(always)]
unsafe fn ZSTD_pthread_mutex_unlock(_a: *mut ZSTD_pthread_mutex_t) {}

#[inline(always)]
unsafe fn ZSTD_pthread_cond_init(_a: *mut ZSTD_pthread_cond_t, _b: *const c_void) -> c_int {
    0
}
#[inline(always)]
unsafe fn ZSTD_pthread_cond_destroy(_a: *mut ZSTD_pthread_cond_t) {}
#[inline(always)]
unsafe fn ZSTD_pthread_cond_wait(_a: *mut ZSTD_pthread_cond_t, _b: *mut ZSTD_pthread_mutex_t) {}
#[inline(always)]
unsafe fn ZSTD_pthread_cond_signal(_a: *mut ZSTD_pthread_cond_t) {}
#[inline(always)]
unsafe fn ZSTD_pthread_cond_broadcast(_a: *mut ZSTD_pthread_cond_t) {}

/* ZSTD_PTHREAD_MUTEX_LOCK(m) -> ZSTD_pthread_mutex_lock(m) (DEBUGLEVEL < 6) */
#[inline(always)]
unsafe fn ZSTD_PTHREAD_MUTEX_LOCK(m: *mut ZSTD_pthread_mutex_t) {
    ZSTD_pthread_mutex_lock(m);
}

/* =====   Buffer Pool   ===== */
/* a single Buffer Pool can be invoked from multiple threads in parallel */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Buffer {
    pub start: *mut c_void,
    pub capacity: size_t,
}

const g_nullBuffer: Buffer = Buffer {
    start: null_mut(),
    capacity: 0,
};

#[repr(C)]
pub struct ZSTDMT_bufferPool {
    pub poolMutex: ZSTD_pthread_mutex_t,
    pub bufferSize: size_t,
    pub totalBuffers: c_uint,
    pub nbBuffers: c_uint,
    pub cMem: ZSTD_customMem,
    pub buffers: *mut Buffer,
}

pub unsafe fn ZSTDMT_freeBufferPool(bufPool: *mut ZSTDMT_bufferPool) {
    if bufPool.is_null() {
        return; /* compatibility with free on NULL */
    }
    if !(*bufPool).buffers.is_null() {
        let mut u: c_uint;
        u = 0;
        while u < (*bufPool).totalBuffers {
            ZSTD_customFree((*(*bufPool).buffers.offset(u as isize)).start, (*bufPool).cMem);
            u += 1;
        }
        ZSTD_customFree((*bufPool).buffers as *mut c_void, (*bufPool).cMem);
    }
    ZSTD_pthread_mutex_destroy(&raw mut (*bufPool).poolMutex);
    ZSTD_customFree(bufPool as *mut c_void, (*bufPool).cMem);
}

pub unsafe fn ZSTDMT_createBufferPool(
    maxNbBuffers: c_uint,
    cMem: ZSTD_customMem,
) -> *mut ZSTDMT_bufferPool {
    let bufPool: *mut ZSTDMT_bufferPool = ZSTD_customCalloc(
        core::mem::size_of::<ZSTDMT_bufferPool>() as size_t,
        cMem,
    ) as *mut ZSTDMT_bufferPool;
    if bufPool.is_null() {
        return null_mut();
    }
    if ZSTD_pthread_mutex_init(&raw mut (*bufPool).poolMutex, null_mut()) != 0 {
        ZSTD_customFree(bufPool as *mut c_void, cMem);
        return null_mut();
    }
    (*bufPool).buffers = ZSTD_customCalloc(
        (maxNbBuffers as size_t).wrapping_mul(core::mem::size_of::<Buffer>() as size_t),
        cMem,
    ) as *mut Buffer;
    if (*bufPool).buffers.is_null() {
        ZSTDMT_freeBufferPool(bufPool);
        return null_mut();
    }
    (*bufPool).bufferSize = 64 * (1 << 10);
    (*bufPool).totalBuffers = maxNbBuffers;
    (*bufPool).nbBuffers = 0;
    (*bufPool).cMem = cMem;
    bufPool
}

/* only works at initialization, not during compression */
pub unsafe fn ZSTDMT_sizeof_bufferPool(bufPool: *mut ZSTDMT_bufferPool) -> size_t {
    let poolSize: size_t = core::mem::size_of::<ZSTDMT_bufferPool>() as size_t;
    let arraySize: size_t =
        ((*bufPool).totalBuffers as size_t).wrapping_mul(core::mem::size_of::<Buffer>() as size_t);
    let mut u: c_uint;
    let mut totalBufferSize: size_t = 0;
    ZSTD_pthread_mutex_lock(&raw mut (*bufPool).poolMutex);
    u = 0;
    while u < (*bufPool).totalBuffers {
        totalBufferSize =
            totalBufferSize.wrapping_add((*(*bufPool).buffers.offset(u as isize)).capacity);
        u += 1;
    }
    ZSTD_pthread_mutex_unlock(&raw mut (*bufPool).poolMutex);

    poolSize.wrapping_add(arraySize).wrapping_add(totalBufferSize)
}

/* ZSTDMT_setBufferSize() */
pub unsafe fn ZSTDMT_setBufferSize(bufPool: *mut ZSTDMT_bufferPool, bSize: size_t) {
    ZSTD_pthread_mutex_lock(&raw mut (*bufPool).poolMutex);
    (*bufPool).bufferSize = bSize;
    ZSTD_pthread_mutex_unlock(&raw mut (*bufPool).poolMutex);
}

pub unsafe fn ZSTDMT_expandBufferPool(
    srcBufPool: *mut ZSTDMT_bufferPool,
    maxNbBuffers: c_uint,
) -> *mut ZSTDMT_bufferPool {
    if srcBufPool.is_null() {
        return null_mut();
    }
    if (*srcBufPool).totalBuffers >= maxNbBuffers {
        /* good enough */
        return srcBufPool;
    }
    /* need a larger buffer pool */
    {
        let cMem: ZSTD_customMem = (*srcBufPool).cMem;
        let bSize: size_t = (*srcBufPool).bufferSize; /* forward parameters */
        let newBufPool: *mut ZSTDMT_bufferPool;
        ZSTDMT_freeBufferPool(srcBufPool);
        newBufPool = ZSTDMT_createBufferPool(maxNbBuffers, cMem);
        if newBufPool.is_null() {
            return newBufPool;
        }
        ZSTDMT_setBufferSize(newBufPool, bSize);
        newBufPool
    }
}

/** ZSTDMT_getBuffer() */
pub unsafe fn ZSTDMT_getBuffer(bufPool: *mut ZSTDMT_bufferPool) -> Buffer {
    let bSize: size_t = (*bufPool).bufferSize;
    ZSTD_pthread_mutex_lock(&raw mut (*bufPool).poolMutex);
    if (*bufPool).nbBuffers != 0 {
        /* try to use an existing buffer */
        (*bufPool).nbBuffers -= 1;
        let buf: Buffer = *(*bufPool).buffers.offset((*bufPool).nbBuffers as isize);
        let availBufferSize: size_t = buf.capacity;
        *(*bufPool).buffers.offset((*bufPool).nbBuffers as isize) = g_nullBuffer;
        if ((availBufferSize >= bSize) as c_int & ((availBufferSize >> 3) <= bSize) as c_int) != 0 {
            /* large enough, but not too much */
            ZSTD_pthread_mutex_unlock(&raw mut (*bufPool).poolMutex);
            return buf;
        }
        /* size conditions not respected : scratch this buffer, create new one */
        ZSTD_customFree(buf.start, (*bufPool).cMem);
    }
    ZSTD_pthread_mutex_unlock(&raw mut (*bufPool).poolMutex);
    /* create new buffer */
    {
        let mut buffer: Buffer = Buffer {
            start: null_mut(),
            capacity: 0,
        };
        let start: *mut c_void = ZSTD_customMalloc(bSize, (*bufPool).cMem);
        buffer.start = start; /* note : start can be NULL if malloc fails ! */
        buffer.capacity = if start.is_null() { 0 } else { bSize };
        buffer
    }
}

/* store buffer for later re-use, up to pool capacity */
pub unsafe fn ZSTDMT_releaseBuffer(bufPool: *mut ZSTDMT_bufferPool, buf: Buffer) {
    if buf.start.is_null() {
        return; /* compatible with release on NULL */
    }
    ZSTD_pthread_mutex_lock(&raw mut (*bufPool).poolMutex);
    if (*bufPool).nbBuffers < (*bufPool).totalBuffers {
        *(*bufPool).buffers.offset((*bufPool).nbBuffers as isize) = buf; /* stored for later use */
        (*bufPool).nbBuffers += 1;
        ZSTD_pthread_mutex_unlock(&raw mut (*bufPool).poolMutex);
        return;
    }
    ZSTD_pthread_mutex_unlock(&raw mut (*bufPool).poolMutex);
    /* Reached bufferPool capacity (note: should not happen) */
    ZSTD_customFree(buf.start, (*bufPool).cMem);
}

/* BUF_POOL_MAX_NB_BUFFERS(nbWorkers) (2*(nbWorkers) + 3) */
#[inline(always)]
fn BUF_POOL_MAX_NB_BUFFERS(nbWorkers: c_uint) -> c_uint {
    2u32.wrapping_mul(nbWorkers).wrapping_add(3)
}

/* SEQ_POOL_MAX_NB_BUFFERS(nbWorkers) (nbWorkers) */
#[inline(always)]
fn SEQ_POOL_MAX_NB_BUFFERS(nbWorkers: c_uint) -> c_uint {
    nbWorkers
}

/* =====   Seq Pool Wrapper   ====== */

pub type ZSTDMT_seqPool = ZSTDMT_bufferPool;

pub unsafe fn ZSTDMT_sizeof_seqPool(seqPool: *mut ZSTDMT_seqPool) -> size_t {
    ZSTDMT_sizeof_bufferPool(seqPool)
}

unsafe fn bufferToSeq(buffer: Buffer) -> RawSeqStore_t {
    let mut seq: RawSeqStore_t = kNullRawSeqStore;
    seq.seq = buffer.start as *mut rawSeq;
    seq.capacity = buffer.capacity / (core::mem::size_of::<rawSeq>() as size_t);
    seq
}

unsafe fn seqToBuffer(seq: RawSeqStore_t) -> Buffer {
    let mut buffer: Buffer = Buffer {
        start: null_mut(),
        capacity: 0,
    };
    buffer.start = seq.seq as *mut c_void;
    buffer.capacity = seq.capacity.wrapping_mul(core::mem::size_of::<rawSeq>() as size_t);
    buffer
}

pub unsafe fn ZSTDMT_getSeq(seqPool: *mut ZSTDMT_seqPool) -> RawSeqStore_t {
    if (*seqPool).bufferSize == 0 {
        return kNullRawSeqStore;
    }
    bufferToSeq(ZSTDMT_getBuffer(seqPool))
}

pub unsafe fn ZSTDMT_releaseSeq(seqPool: *mut ZSTDMT_seqPool, seq: RawSeqStore_t) {
    ZSTDMT_releaseBuffer(seqPool, seqToBuffer(seq));
}

pub unsafe fn ZSTDMT_setNbSeq(seqPool: *mut ZSTDMT_seqPool, nbSeq: size_t) {
    ZSTDMT_setBufferSize(seqPool, nbSeq.wrapping_mul(core::mem::size_of::<rawSeq>() as size_t));
}

pub unsafe fn ZSTDMT_createSeqPool(nbWorkers: c_uint, cMem: ZSTD_customMem) -> *mut ZSTDMT_seqPool {
    let seqPool: *mut ZSTDMT_seqPool =
        ZSTDMT_createBufferPool(SEQ_POOL_MAX_NB_BUFFERS(nbWorkers), cMem);
    if seqPool.is_null() {
        return null_mut();
    }
    ZSTDMT_setNbSeq(seqPool, 0);
    seqPool
}

pub unsafe fn ZSTDMT_freeSeqPool(seqPool: *mut ZSTDMT_seqPool) {
    ZSTDMT_freeBufferPool(seqPool);
}

pub unsafe fn ZSTDMT_expandSeqPool(pool: *mut ZSTDMT_seqPool, nbWorkers: U32) -> *mut ZSTDMT_seqPool {
    ZSTDMT_expandBufferPool(pool, SEQ_POOL_MAX_NB_BUFFERS(nbWorkers))
}

/* =====   CCtx Pool   ===== */
/* a single CCtx Pool can be invoked from multiple threads in parallel */

#[repr(C)]
pub struct ZSTDMT_CCtxPool {
    pub poolMutex: ZSTD_pthread_mutex_t,
    pub totalCCtx: c_int,
    pub availCCtx: c_int,
    pub cMem: ZSTD_customMem,
    pub cctxs: *mut *mut ZSTD_CCtx,
}

/* note : all CCtx borrowed from the pool must be reverted back to the pool _before_ freeing the pool */
pub unsafe fn ZSTDMT_freeCCtxPool(pool: *mut ZSTDMT_CCtxPool) {
    if pool.is_null() {
        return;
    }
    ZSTD_pthread_mutex_destroy(&raw mut (*pool).poolMutex);
    if !(*pool).cctxs.is_null() {
        let mut cid: c_int;
        cid = 0;
        while cid < (*pool).totalCCtx {
            ZSTD_freeCCtx(*(*pool).cctxs.offset(cid as isize)); /* free compatible with NULL */
            cid += 1;
        }
        ZSTD_customFree((*pool).cctxs as *mut c_void, (*pool).cMem);
    }
    ZSTD_customFree(pool as *mut c_void, (*pool).cMem);
}

/* ZSTDMT_createCCtxPool() : implies nbWorkers >= 1 */
pub unsafe fn ZSTDMT_createCCtxPool(nbWorkers: c_int, cMem: ZSTD_customMem) -> *mut ZSTDMT_CCtxPool {
    let cctxPool: *mut ZSTDMT_CCtxPool = ZSTD_customCalloc(
        core::mem::size_of::<ZSTDMT_CCtxPool>() as size_t,
        cMem,
    ) as *mut ZSTDMT_CCtxPool;
    if cctxPool.is_null() {
        return null_mut();
    }
    if ZSTD_pthread_mutex_init(&raw mut (*cctxPool).poolMutex, null_mut()) != 0 {
        ZSTD_customFree(cctxPool as *mut c_void, cMem);
        return null_mut();
    }
    (*cctxPool).totalCCtx = nbWorkers;
    (*cctxPool).cctxs = ZSTD_customCalloc(
        (nbWorkers as size_t).wrapping_mul(core::mem::size_of::<*mut ZSTD_CCtx>() as size_t),
        cMem,
    ) as *mut *mut ZSTD_CCtx;
    if (*cctxPool).cctxs.is_null() {
        ZSTDMT_freeCCtxPool(cctxPool);
        return null_mut();
    }
    (*cctxPool).cMem = cMem;
    *(*cctxPool).cctxs.offset(0) = ZSTD_createCCtx_advanced(cMem);
    if (*(*cctxPool).cctxs.offset(0)).is_null() {
        ZSTDMT_freeCCtxPool(cctxPool);
        return null_mut();
    }
    (*cctxPool).availCCtx = 1; /* at least one cctx for single-thread mode */
    cctxPool
}

pub unsafe fn ZSTDMT_expandCCtxPool(
    srcPool: *mut ZSTDMT_CCtxPool,
    nbWorkers: c_int,
) -> *mut ZSTDMT_CCtxPool {
    if srcPool.is_null() {
        return null_mut();
    }
    if nbWorkers <= (*srcPool).totalCCtx {
        return srcPool; /* good enough */
    }
    /* need a larger cctx pool */
    {
        let cMem: ZSTD_customMem = (*srcPool).cMem;
        ZSTDMT_freeCCtxPool(srcPool);
        ZSTDMT_createCCtxPool(nbWorkers, cMem)
    }
}

/* only works during initialization phase, not during compression */
pub unsafe fn ZSTDMT_sizeof_CCtxPool(cctxPool: *mut ZSTDMT_CCtxPool) -> size_t {
    ZSTD_pthread_mutex_lock(&raw mut (*cctxPool).poolMutex);
    {
        let nbWorkers: c_uint = (*cctxPool).totalCCtx as c_uint;
        let poolSize: size_t = core::mem::size_of::<ZSTDMT_CCtxPool>() as size_t;
        let arraySize: size_t = ((*cctxPool).totalCCtx as size_t)
            .wrapping_mul(core::mem::size_of::<*mut ZSTD_CCtx>() as size_t);
        let mut totalCCtxSize: size_t = 0;
        let mut u: c_uint;
        u = 0;
        while u < nbWorkers {
            totalCCtxSize =
                totalCCtxSize.wrapping_add(ZSTD_sizeof_CCtx(*(*cctxPool).cctxs.offset(u as isize)));
            u += 1;
        }
        ZSTD_pthread_mutex_unlock(&raw mut (*cctxPool).poolMutex);
        poolSize.wrapping_add(arraySize).wrapping_add(totalCCtxSize)
    }
}

pub unsafe fn ZSTDMT_getCCtx(cctxPool: *mut ZSTDMT_CCtxPool) -> *mut ZSTD_CCtx {
    ZSTD_pthread_mutex_lock(&raw mut (*cctxPool).poolMutex);
    if (*cctxPool).availCCtx != 0 {
        (*cctxPool).availCCtx -= 1;
        {
            let cctx: *mut ZSTD_CCtx = *(*cctxPool).cctxs.offset((*cctxPool).availCCtx as isize);
            ZSTD_pthread_mutex_unlock(&raw mut (*cctxPool).poolMutex);
            return cctx;
        }
    }
    ZSTD_pthread_mutex_unlock(&raw mut (*cctxPool).poolMutex);
    ZSTD_createCCtx_advanced((*cctxPool).cMem) /* note : can be NULL, when creation fails ! */
}

pub unsafe fn ZSTDMT_releaseCCtx(pool: *mut ZSTDMT_CCtxPool, cctx: *mut ZSTD_CCtx) {
    if cctx.is_null() {
        return; /* compatibility with release on NULL */
    }
    ZSTD_pthread_mutex_lock(&raw mut (*pool).poolMutex);
    if (*pool).availCCtx < (*pool).totalCCtx {
        *(*pool).cctxs.offset((*pool).availCCtx as isize) = cctx;
        (*pool).availCCtx += 1;
    } else {
        /* pool overflow : should not happen, since totalCCtx==nbWorkers */
        ZSTD_freeCCtx(cctx);
    }
    ZSTD_pthread_mutex_unlock(&raw mut (*pool).poolMutex);
}

/* ====   Serial State   ==== */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Range {
    pub start: *const c_void,
    pub size: size_t,
}

#[repr(C)]
pub struct SerialState {
    /* All variables in the struct are protected by mutex. */
    pub mutex: ZSTD_pthread_mutex_t,
    pub cond: ZSTD_pthread_cond_t,
    pub params: ZSTD_CCtx_params,
    pub ldmState: ldmState_t,
    pub xxhState: XXH64_state_t,
    pub nextJobID: c_uint,
    /* Protects ldmWindow.
     * Must be acquired after the main mutex when acquiring both.
     */
    pub ldmWindowMutex: ZSTD_pthread_mutex_t,
    pub ldmWindowCond: ZSTD_pthread_cond_t, /* Signaled when ldmWindow is updated */
    pub ldmWindow: ZSTD_window_t,           /* A thread-safe copy of ldmState.window */
}

pub unsafe fn ZSTDMT_serialState_reset(
    serialState: *mut SerialState,
    seqPool: *mut ZSTDMT_seqPool,
    mut params: ZSTD_CCtx_params,
    jobSize: size_t,
    dict: *const c_void,
    dictSize: size_t,
    dictContentType: ZSTD_dictContentType_e,
) -> c_int {
    /* Adjust parameters */
    if params.ldmParams.enableLdm == ZSTD_ps_enable {
        ZSTD_ldm_adjustParameters(&raw mut params.ldmParams, &raw const params.cParams);
    } else {
        ZSTD_memset(
            &raw mut params.ldmParams as *mut u8,
            0,
            core::mem::size_of::<ldmParams_t>() as size_t,
        );
    }
    (*serialState).nextJobID = 0;
    if params.fParams.checksumFlag != 0 {
        ZSTD_XXH64_reset(&raw mut (*serialState).xxhState, 0);
    }
    if params.ldmParams.enableLdm == ZSTD_ps_enable {
        let cMem: ZSTD_customMem = params.customMem;
        let hashLog: c_uint = params.ldmParams.hashLog;
        let hashSize: size_t =
            ((1usize << hashLog) as size_t).wrapping_mul(core::mem::size_of::<ldmEntry_t>() as size_t);
        let bucketLog: c_uint = params.ldmParams.hashLog - params.ldmParams.bucketSizeLog;
        let prevBucketLog: c_uint =
            (*serialState).params.ldmParams.hashLog - (*serialState).params.ldmParams.bucketSizeLog;
        let numBuckets: size_t = 1usize << bucketLog;
        /* Size the seq pool tables */
        ZSTDMT_setNbSeq(seqPool, ZSTD_ldm_getMaxNbSeq(params.ldmParams, jobSize));
        /* Reset the window */
        ZSTD_window_init(&raw mut (*serialState).ldmState.window);
        /* Resize tables and output space if necessary. */
        if (*serialState).ldmState.hashTable.is_null()
            || (*serialState).params.ldmParams.hashLog < hashLog
        {
            ZSTD_customFree((*serialState).ldmState.hashTable as *mut c_void, cMem);
            (*serialState).ldmState.hashTable =
                ZSTD_customMalloc(hashSize, cMem) as *mut ldmEntry_t;
        }
        if (*serialState).ldmState.bucketOffsets.is_null() || prevBucketLog < bucketLog {
            ZSTD_customFree((*serialState).ldmState.bucketOffsets as *mut c_void, cMem);
            (*serialState).ldmState.bucketOffsets = ZSTD_customMalloc(numBuckets, cMem) as *mut BYTE;
        }
        if (*serialState).ldmState.hashTable.is_null()
            || (*serialState).ldmState.bucketOffsets.is_null()
        {
            return 1;
        }
        /* Zero the tables */
        ZSTD_memset((*serialState).ldmState.hashTable as *mut u8, 0, hashSize);
        ZSTD_memset((*serialState).ldmState.bucketOffsets, 0, numBuckets);

        /* Update window state and fill hash table with dict */
        (*serialState).ldmState.loadedDictEnd = 0;
        if dictSize > 0 {
            if dictContentType == ZSTD_dct_rawContent {
                let dictEnd: *const BYTE = (dict as *const BYTE).wrapping_add(dictSize);
                ZSTD_window_update(
                    &raw mut (*serialState).ldmState.window,
                    dict,
                    dictSize,
                    /* forceNonContiguous */ 0,
                );
                ZSTD_ldm_fillHashTable(
                    &raw mut (*serialState).ldmState,
                    dict as *const BYTE,
                    dictEnd,
                    &raw const params.ldmParams,
                );
                (*serialState).ldmState.loadedDictEnd = if params.forceWindow != 0 {
                    0
                } else {
                    dictEnd.offset_from((*serialState).ldmState.window.base) as U32
                };
            } else {
                /* don't even load anything */
            }
        }

        /* Initialize serialState's copy of ldmWindow. */
        (*serialState).ldmWindow = (*serialState).ldmState.window;
    }

    (*serialState).params = core::ptr::read(&raw const params);
    (*serialState).params.jobSize = (jobSize as U32) as size_t;
    0
}

pub unsafe fn ZSTDMT_serialState_init(serialState: *mut SerialState) -> c_int {
    let mut initError: c_int = 0;
    ZSTD_memset(
        serialState as *mut u8,
        0,
        core::mem::size_of::<SerialState>() as size_t,
    );
    initError |= ZSTD_pthread_mutex_init(&raw mut (*serialState).mutex, null_mut());
    initError |= ZSTD_pthread_cond_init(&raw mut (*serialState).cond, null_mut());
    initError |= ZSTD_pthread_mutex_init(&raw mut (*serialState).ldmWindowMutex, null_mut());
    initError |= ZSTD_pthread_cond_init(&raw mut (*serialState).ldmWindowCond, null_mut());
    initError
}

pub unsafe fn ZSTDMT_serialState_free(serialState: *mut SerialState) {
    let cMem: ZSTD_customMem = (*serialState).params.customMem;
    ZSTD_pthread_mutex_destroy(&raw mut (*serialState).mutex);
    ZSTD_pthread_cond_destroy(&raw mut (*serialState).cond);
    ZSTD_pthread_mutex_destroy(&raw mut (*serialState).ldmWindowMutex);
    ZSTD_pthread_cond_destroy(&raw mut (*serialState).ldmWindowCond);
    ZSTD_customFree((*serialState).ldmState.hashTable as *mut c_void, cMem);
    ZSTD_customFree((*serialState).ldmState.bucketOffsets as *mut c_void, cMem);
}

pub unsafe fn ZSTDMT_serialState_genSequences(
    serialState: *mut SerialState,
    seqStore: *mut RawSeqStore_t,
    src: Range,
    jobID: c_uint,
) {
    /* Wait for our turn */
    ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*serialState).mutex);
    while (*serialState).nextJobID < jobID {
        ZSTD_pthread_cond_wait(&raw mut (*serialState).cond, &raw mut (*serialState).mutex);
    }
    /* A future job may error and skip our job */
    if (*serialState).nextJobID == jobID {
        /* It is now our turn, do any processing necessary */
        if (*serialState).params.ldmParams.enableLdm == ZSTD_ps_enable {
            let error: size_t;
            ZSTD_window_update(
                &raw mut (*serialState).ldmState.window,
                src.start,
                src.size,
                /* forceNonContiguous */ 0,
            );
            error = ZSTD_ldm_generateSequences(
                &raw mut (*serialState).ldmState,
                seqStore,
                &raw const (*serialState).params.ldmParams,
                src.start,
                src.size,
            );
            /* We provide a large enough buffer to never fail. */
            let _ = error;
            /* Update ldmWindow to match the ldmState.window and signal the main
             * thread if it is waiting for a buffer.
             */
            ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*serialState).ldmWindowMutex);
            (*serialState).ldmWindow = (*serialState).ldmState.window;
            ZSTD_pthread_cond_signal(&raw mut (*serialState).ldmWindowCond);
            ZSTD_pthread_mutex_unlock(&raw mut (*serialState).ldmWindowMutex);
        }
        if (*serialState).params.fParams.checksumFlag != 0 && src.size > 0 {
            ZSTD_XXH64_update(&raw mut (*serialState).xxhState, src.start, src.size);
        }
    }
    /* Now it is the next jobs turn */
    (*serialState).nextJobID += 1;
    ZSTD_pthread_cond_broadcast(&raw mut (*serialState).cond);
    ZSTD_pthread_mutex_unlock(&raw mut (*serialState).mutex);
}

pub unsafe fn ZSTDMT_serialState_applySequences(
    serialState: *const SerialState, /* just for an assert() check */
    jobCCtx: *mut ZSTD_CCtx,
    seqStore: *const RawSeqStore_t,
) {
    if (*seqStore).size > 0 {
        let _ = serialState;
        ZSTD_referenceExternalSequences(jobCCtx, (*seqStore).seq, (*seqStore).size);
    }
}

pub unsafe fn ZSTDMT_serialState_ensureFinished(
    serialState: *mut SerialState,
    jobID: c_uint,
    cSize: size_t,
) {
    ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*serialState).mutex);
    if (*serialState).nextJobID <= jobID {
        let _ = cSize;
        (*serialState).nextJobID = jobID + 1;
        ZSTD_pthread_cond_broadcast(&raw mut (*serialState).cond);

        ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*serialState).ldmWindowMutex);
        ZSTD_window_clear(&raw mut (*serialState).ldmWindow);
        ZSTD_pthread_cond_signal(&raw mut (*serialState).ldmWindowCond);
        ZSTD_pthread_mutex_unlock(&raw mut (*serialState).ldmWindowMutex);
    }
    ZSTD_pthread_mutex_unlock(&raw mut (*serialState).mutex);
}

/* ------------------------------------------ */
/* =====          Worker thread         ===== */
/* ------------------------------------------ */

const kNullRange: Range = Range {
    start: core::ptr::null(),
    size: 0,
};

#[repr(C)]
pub struct ZSTDMT_jobDescription {
    pub consumed: size_t, /* SHARED */
    pub cSize: size_t,    /* SHARED */
    pub job_mutex: ZSTD_pthread_mutex_t,
    pub job_cond: ZSTD_pthread_cond_t,
    pub cctxPool: *mut ZSTDMT_CCtxPool,
    pub bufPool: *mut ZSTDMT_bufferPool,
    pub seqPool: *mut ZSTDMT_seqPool,
    pub serial: *mut SerialState,
    pub dstBuff: Buffer,
    pub prefix: Range,
    pub src: Range,
    pub jobID: c_uint,
    pub firstJob: c_uint,
    pub lastJob: c_uint,
    pub params: ZSTD_CCtx_params,
    pub cdict: *const ZSTD_CDict,
    pub fullFrameSize: c_ulonglong,
    pub dstFlushed: size_t,          /* used only by mtctx */
    pub frameChecksumNeeded: c_uint, /* used only by mtctx */
}

/* ZSTDMT_compressionJob() is a POOL_function type */
pub unsafe extern "C" fn ZSTDMT_compressionJob(jobDescription: *mut c_void) {
    let job: *mut ZSTDMT_jobDescription = jobDescription as *mut ZSTDMT_jobDescription;
    /* do not modify job->params ! copy it, modify the copy */
    let mut jobParams: ZSTD_CCtx_params = core::ptr::read(&raw const (*job).params);
    let cctx: *mut ZSTD_CCtx = ZSTDMT_getCCtx((*job).cctxPool);
    let mut rawSeqStore: RawSeqStore_t = ZSTDMT_getSeq((*job).seqPool);
    let mut dstBuff: Buffer = (*job).dstBuff;
    let mut lastCBlockSize: size_t = 0;

    /* JOB_ERROR(e) : set job->cSize=e then goto _endJob. Modelled with a labelled
     * block whose completion falls through into the _endJob code. */
    'endJob: {
        /* resources */
        if cctx.is_null() {
            ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*job).job_mutex);
            (*job).cSize = ERROR(ZSTD_error_memory_allocation);
            ZSTD_pthread_mutex_unlock(&raw mut (*job).job_mutex);
            break 'endJob;
        }
        if dstBuff.start.is_null() {
            /* streaming job : doesn't provide a dstBuffer */
            dstBuff = ZSTDMT_getBuffer((*job).bufPool);
            if dstBuff.start.is_null() {
                ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*job).job_mutex);
                (*job).cSize = ERROR(ZSTD_error_memory_allocation);
                ZSTD_pthread_mutex_unlock(&raw mut (*job).job_mutex);
                break 'endJob;
            }
            (*job).dstBuff = dstBuff; /* this value can be read in ZSTDMT_flush */
        }
        if jobParams.ldmParams.enableLdm == ZSTD_ps_enable && rawSeqStore.seq.is_null() {
            ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*job).job_mutex);
            (*job).cSize = ERROR(ZSTD_error_memory_allocation);
            ZSTD_pthread_mutex_unlock(&raw mut (*job).job_mutex);
            break 'endJob;
        }

        /* Don't compute the checksum for chunks, since we compute it externally,
         * but write it in the header. */
        if (*job).jobID != 0 {
            jobParams.fParams.checksumFlag = 0;
        }
        /* Don't run LDM for the chunks, since we handle it externally */
        jobParams.ldmParams.enableLdm = ZSTD_ps_disable;
        /* Correct nbWorkers to 0. */
        jobParams.nbWorkers = 0;

        /* init */

        /* Perform serial step as early as possible */
        ZSTDMT_serialState_genSequences(
            (*job).serial,
            &raw mut rawSeqStore,
            (*job).src,
            (*job).jobID,
        );

        if !(*job).cdict.is_null() {
            let initError: size_t = ZSTD_compressBegin_advanced_internal(
                cctx,
                null_mut(),
                0,
                ZSTD_dct_auto,
                ZSTD_dtlm_fast,
                (*job).cdict,
                &raw const jobParams,
                (*job).fullFrameSize,
            );
            if ZSTD_isError(initError) != 0 {
                ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*job).job_mutex);
                (*job).cSize = initError;
                ZSTD_pthread_mutex_unlock(&raw mut (*job).job_mutex);
                break 'endJob;
            }
        } else {
            let pledgedSrcSize: U64 = if (*job).firstJob != 0 {
                (*job).fullFrameSize
            } else {
                (*job).src.size as U64
            };
            {
                let forceWindowError: size_t = ZSTD_CCtxParams_setParameter(
                    &raw mut jobParams,
                    ZSTD_c_forceMaxWindow,
                    ((*job).firstJob == 0) as c_int,
                );
                if ZSTD_isError(forceWindowError) != 0 {
                    ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*job).job_mutex);
                    (*job).cSize = forceWindowError;
                    ZSTD_pthread_mutex_unlock(&raw mut (*job).job_mutex);
                    break 'endJob;
                }
            }
            if (*job).firstJob == 0 {
                let err: size_t =
                    ZSTD_CCtxParams_setParameter(&raw mut jobParams, ZSTD_c_deterministicRefPrefix, 0);
                if ZSTD_isError(err) != 0 {
                    ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*job).job_mutex);
                    (*job).cSize = err;
                    ZSTD_pthread_mutex_unlock(&raw mut (*job).job_mutex);
                    break 'endJob;
                }
            }
            {
                let initError: size_t = ZSTD_compressBegin_advanced_internal(
                    cctx,
                    (*job).prefix.start,
                    (*job).prefix.size,
                    ZSTD_dct_rawContent,
                    ZSTD_dtlm_fast,
                    null_mut(), /*cdict*/
                    &raw const jobParams,
                    pledgedSrcSize,
                );
                if ZSTD_isError(initError) != 0 {
                    ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*job).job_mutex);
                    (*job).cSize = initError;
                    ZSTD_pthread_mutex_unlock(&raw mut (*job).job_mutex);
                    break 'endJob;
                }
            }
        }

        /* External Sequences can only be applied after CCtx initialization */
        ZSTDMT_serialState_applySequences((*job).serial, cctx, &raw const rawSeqStore);

        if (*job).firstJob == 0 {
            /* flush and overwrite frame header when it's not first job */
            let hSize: size_t = ZSTD_compressContinue_public(
                cctx,
                dstBuff.start,
                dstBuff.capacity,
                (*job).src.start,
                0,
            );
            if ZSTD_isError(hSize) != 0 {
                ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*job).job_mutex);
                (*job).cSize = hSize;
                ZSTD_pthread_mutex_unlock(&raw mut (*job).job_mutex);
                break 'endJob;
            }
            ZSTD_invalidateRepCodes(cctx);
        }

        /* compress the entire job by smaller chunks, for better granularity */
        {
            let chunkSize: size_t = (4 * ZSTD_BLOCKSIZE_MAX) as size_t;
            let nbChunks: c_int =
                (((*job).src.size + (chunkSize - 1)) / chunkSize) as c_int;
            let mut ip: *const BYTE = (*job).src.start as *const BYTE;
            let ostart: *mut BYTE = dstBuff.start as *mut BYTE;
            let mut op: *mut BYTE = ostart;
            let oend: *mut BYTE = op.wrapping_add(dstBuff.capacity);
            let mut chunkNb: c_int;
            chunkNb = 1;
            while chunkNb < nbChunks {
                let cSize: size_t = ZSTD_compressContinue_public(
                    cctx,
                    op as *mut c_void,
                    oend.offset_from(op) as size_t,
                    ip as *const c_void,
                    chunkSize,
                );
                if ZSTD_isError(cSize) != 0 {
                    ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*job).job_mutex);
                    (*job).cSize = cSize;
                    ZSTD_pthread_mutex_unlock(&raw mut (*job).job_mutex);
                    break 'endJob;
                }
                ip = ip.wrapping_add(chunkSize);
                op = op.wrapping_add(cSize);
                /* stats */
                ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*job).job_mutex);
                (*job).cSize = (*job).cSize.wrapping_add(cSize);
                (*job).consumed = chunkSize.wrapping_mul(chunkNb as size_t);
                ZSTD_pthread_cond_signal(&raw mut (*job).job_cond); /* warns some more data is ready to be flushed */
                ZSTD_pthread_mutex_unlock(&raw mut (*job).job_mutex);
                chunkNb += 1;
            }
            /* last block */
            if ((nbChunks > 0) as c_int | (*job).lastJob as c_int) != 0
            /*must output a "last block" flag*/
            {
                let lastBlockSize1: size_t = (*job).src.size & (chunkSize - 1);
                let lastBlockSize: size_t = if ((lastBlockSize1 == 0) as c_int
                    & ((*job).src.size >= chunkSize) as c_int)
                    != 0
                {
                    chunkSize
                } else {
                    lastBlockSize1
                };
                let cSize: size_t = if (*job).lastJob != 0 {
                    ZSTD_compressEnd_public(
                        cctx,
                        op as *mut c_void,
                        oend.offset_from(op) as size_t,
                        ip as *const c_void,
                        lastBlockSize,
                    )
                } else {
                    ZSTD_compressContinue_public(
                        cctx,
                        op as *mut c_void,
                        oend.offset_from(op) as size_t,
                        ip as *const c_void,
                        lastBlockSize,
                    )
                };
                if ZSTD_isError(cSize) != 0 {
                    ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*job).job_mutex);
                    (*job).cSize = cSize;
                    ZSTD_pthread_mutex_unlock(&raw mut (*job).job_mutex);
                    break 'endJob;
                }
                lastCBlockSize = cSize;
            }
        }
        /* if (!job->firstJob) : assert only, dropped at DEBUGLEVEL 0 */
        ZSTD_CCtx_trace(cctx, 0);
    } /* 'endJob: */

    /* _endJob: */
    ZSTDMT_serialState_ensureFinished((*job).serial, (*job).jobID, (*job).cSize);
    /* release resources */
    ZSTDMT_releaseSeq((*job).seqPool, rawSeqStore);
    ZSTDMT_releaseCCtx((*job).cctxPool, cctx);
    /* report */
    ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*job).job_mutex);
    (*job).cSize = (*job).cSize.wrapping_add(lastCBlockSize);
    (*job).consumed = (*job).src.size; /* when job->consumed == job->src.size , compression job is presumed completed */
    ZSTD_pthread_cond_signal(&raw mut (*job).job_cond);
    ZSTD_pthread_mutex_unlock(&raw mut (*job).job_mutex);
}

/* ------------------------------------------ */
/* =====   Multi-threaded compression   ===== */
/* ------------------------------------------ */

#[repr(C)]
pub struct InBuff_t {
    pub prefix: Range, /* read-only non-owned prefix buffer */
    pub buffer: Buffer,
    pub filled: size_t,
}

#[repr(C)]
pub struct RoundBuff_t {
    pub buffer: *mut BYTE,
    pub capacity: size_t,
    pub pos: size_t,
}

const kNullRoundBuff: RoundBuff_t = RoundBuff_t {
    buffer: null_mut(),
    capacity: 0,
    pos: 0,
};

pub const RSYNC_LENGTH: size_t = 32;
/* RSYNC_MIN_BLOCK_LOG ZSTD_BLOCKSIZELOG_MAX ; RSYNC_MIN_BLOCK_SIZE (1<<RSYNC_MIN_BLOCK_LOG) */
pub const RSYNC_MIN_BLOCK_LOG: c_uint = ZSTD_BLOCKSIZELOG_MAX;
pub const RSYNC_MIN_BLOCK_SIZE: size_t = 1usize << RSYNC_MIN_BLOCK_LOG;

#[repr(C)]
pub struct RSyncState_t {
    pub hash: U64,
    pub hitMask: U64,
    pub primePower: U64,
}

#[repr(C)]
pub struct ZSTDMT_CCtx_s {
    pub factory: *mut POOL_ctx,
    pub jobs: *mut ZSTDMT_jobDescription,
    pub bufPool: *mut ZSTDMT_bufferPool,
    pub cctxPool: *mut ZSTDMT_CCtxPool,
    pub seqPool: *mut ZSTDMT_seqPool,
    pub params: ZSTD_CCtx_params,
    pub targetSectionSize: size_t,
    pub targetPrefixSize: size_t,
    pub jobReady: c_int, /* 1 => one job is already prepared */
    pub inBuff: InBuff_t,
    pub roundBuff: RoundBuff_t,
    pub serial: SerialState,
    pub rsync: RSyncState_t,
    pub jobIDMask: c_uint,
    pub doneJobID: c_uint,
    pub nextJobID: c_uint,
    pub frameEnded: c_uint,
    pub allJobsCompleted: c_uint,
    pub frameContentSize: c_ulonglong,
    pub consumed: c_ulonglong,
    pub produced: c_ulonglong,
    pub cMem: ZSTD_customMem,
    pub cdictLocal: *mut ZSTD_CDict,
    pub cdict: *const ZSTD_CDict,
    /* unsigned providedFactory: 1; -> bitfield. Modelled as a full c_uint field.
     * mtctx is never constructed in this build, so exact bitfield layout is
     * unobservable, but we keep a field so struct size is consistent. */
    pub providedFactory: c_uint,
}

/* The header declares `typedef struct ZSTDMT_CCtx_s ZSTDMT_CCtx;`.
 * Elsewhere (zstd_compress_internal.rs) `ZSTDMT_CCtx` is an OPAQUE enum so that
 * `*mut ZSTDMT_CCtx` type-checks. The 9 exported entry points below use the
 * opaque `*mut ZSTD_opaque_ZSTDMT_CCtx` pointer type at the ABI boundary and
 * cast to `*mut ZSTDMT_CCtx_s` internally. */
type ZSTDMT_CCtx = ZSTDMT_CCtx_s;

/* Opaque alias matching the one declared in zstd_compress_internal.rs, used only
 * as the ABI pointee type of the exported functions. */
use crate::compress::zstd_compress_internal::ZSTDMT_CCtx as OpaqueZSTDMT_CCtx;

pub unsafe fn ZSTDMT_freeJobsTable(
    jobTable: *mut ZSTDMT_jobDescription,
    nbJobs: U32,
    cMem: ZSTD_customMem,
) {
    let mut jobNb: U32;
    if jobTable.is_null() {
        return;
    }
    jobNb = 0;
    while jobNb < nbJobs {
        ZSTD_pthread_mutex_destroy(&raw mut (*jobTable.offset(jobNb as isize)).job_mutex);
        ZSTD_pthread_cond_destroy(&raw mut (*jobTable.offset(jobNb as isize)).job_cond);
        jobNb += 1;
    }
    ZSTD_customFree(jobTable as *mut c_void, cMem);
}

/* ZSTDMT_allocJobsTable() */
pub unsafe fn ZSTDMT_createJobsTable(
    nbJobsPtr: *mut U32,
    cMem: ZSTD_customMem,
) -> *mut ZSTDMT_jobDescription {
    let nbJobsLog2: U32 = ZSTD_highbit32(*nbJobsPtr) + 1;
    let nbJobs: U32 = 1 << nbJobsLog2;
    let mut jobNb: U32;
    let jobTable: *mut ZSTDMT_jobDescription = ZSTD_customCalloc(
        (nbJobs as size_t).wrapping_mul(core::mem::size_of::<ZSTDMT_jobDescription>() as size_t),
        cMem,
    ) as *mut ZSTDMT_jobDescription;
    let mut initError: c_int = 0;
    if jobTable.is_null() {
        return null_mut();
    }
    *nbJobsPtr = nbJobs;
    jobNb = 0;
    while jobNb < nbJobs {
        initError |=
            ZSTD_pthread_mutex_init(&raw mut (*jobTable.offset(jobNb as isize)).job_mutex, null_mut());
        initError |=
            ZSTD_pthread_cond_init(&raw mut (*jobTable.offset(jobNb as isize)).job_cond, null_mut());
        jobNb += 1;
    }
    if initError != 0 {
        ZSTDMT_freeJobsTable(jobTable, nbJobs, cMem);
        return null_mut();
    }
    jobTable
}

pub unsafe fn ZSTDMT_expandJobsTable(mtctx: *mut ZSTDMT_CCtx, nbWorkers: U32) -> size_t {
    let mut nbJobs: U32 = nbWorkers + 2;
    if nbJobs > (*mtctx).jobIDMask + 1 {
        /* need more job capacity */
        ZSTDMT_freeJobsTable((*mtctx).jobs, (*mtctx).jobIDMask + 1, (*mtctx).cMem);
        (*mtctx).jobIDMask = 0;
        (*mtctx).jobs = ZSTDMT_createJobsTable(&raw mut nbJobs, (*mtctx).cMem);
        if (*mtctx).jobs.is_null() {
            return ERROR(ZSTD_error_memory_allocation);
        }
        (*mtctx).jobIDMask = nbJobs - 1;
    }
    0
}

/* ZSTDMT_CCtxParam_setNbWorkers() : Internal use only */
pub unsafe fn ZSTDMT_CCtxParam_setNbWorkers(
    params: *mut ZSTD_CCtx_params,
    nbWorkers: c_uint,
) -> size_t {
    ZSTD_CCtxParams_setParameter(params, ZSTD_c_nbWorkers, nbWorkers as c_int)
}

pub unsafe fn ZSTDMT_createCCtx_advanced_internal(
    mut nbWorkers: c_uint,
    cMem: ZSTD_customMem,
    pool: *mut ZSTD_threadPool,
) -> *mut ZSTDMT_CCtx {
    let mtctx: *mut ZSTDMT_CCtx;
    let mut nbJobs: U32 = nbWorkers + 2;
    let initError: c_int;

    if nbWorkers < 1 {
        return null_mut();
    }
    nbWorkers = MIN(nbWorkers as size_t, ZSTDMT_NBWORKERS_MAX as size_t) as c_uint;
    if ((cMem.customAlloc.is_some()) as c_int ^ (cMem.customFree.is_some()) as c_int) != 0 {
        /* invalid custom allocator */
        return null_mut();
    }

    mtctx = ZSTD_customCalloc(core::mem::size_of::<ZSTDMT_CCtx>() as size_t, cMem)
        as *mut ZSTDMT_CCtx;
    if mtctx.is_null() {
        return null_mut();
    }
    ZSTDMT_CCtxParam_setNbWorkers(&raw mut (*mtctx).params, nbWorkers);
    (*mtctx).cMem = cMem;
    (*mtctx).allJobsCompleted = 1;
    if !pool.is_null() {
        (*mtctx).factory = pool;
        (*mtctx).providedFactory = 1;
    } else {
        (*mtctx).factory = POOL_create_advanced(nbWorkers as size_t, 0, cMem);
        (*mtctx).providedFactory = 0;
    }
    (*mtctx).jobs = ZSTDMT_createJobsTable(&raw mut nbJobs, cMem);
    (*mtctx).jobIDMask = nbJobs - 1;
    (*mtctx).bufPool = ZSTDMT_createBufferPool(BUF_POOL_MAX_NB_BUFFERS(nbWorkers), cMem);
    (*mtctx).cctxPool = ZSTDMT_createCCtxPool(nbWorkers as c_int, cMem);
    (*mtctx).seqPool = ZSTDMT_createSeqPool(nbWorkers, cMem);
    initError = ZSTDMT_serialState_init(&raw mut (*mtctx).serial);
    (*mtctx).roundBuff = kNullRoundBuff;
    if ((*mtctx).factory.is_null() as c_int
        | (*mtctx).jobs.is_null() as c_int
        | (*mtctx).bufPool.is_null() as c_int
        | (*mtctx).cctxPool.is_null() as c_int
        | (*mtctx).seqPool.is_null() as c_int
        | initError)
        != 0
    {
        ZSTDMT_freeCCtx(mtctx as *mut OpaqueZSTDMT_CCtx);
        return null_mut();
    }
    mtctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_createCCtx_advanced(
    nbWorkers: c_uint,
    cMem: ZSTD_customMem,
    pool: *mut ZSTD_threadPool,
) -> *mut OpaqueZSTDMT_CCtx {
    /* ZSTD_MULTITHREAD not defined */
    let _ = nbWorkers;
    let _ = cMem;
    let _ = pool;
    null_mut()
}

/* ZSTDMT_releaseAllJobResources() : note : ensure all workers are killed first ! */
pub unsafe fn ZSTDMT_releaseAllJobResources(mtctx: *mut ZSTDMT_CCtx) {
    let mut jobID: c_uint;
    jobID = 0;
    while jobID <= (*mtctx).jobIDMask {
        /* Copy the mutex/cond out */
        let mutex: ZSTD_pthread_mutex_t = (*(*mtctx).jobs.offset(jobID as isize)).job_mutex;
        let cond: ZSTD_pthread_cond_t = (*(*mtctx).jobs.offset(jobID as isize)).job_cond;

        ZSTDMT_releaseBuffer((*mtctx).bufPool, (*(*mtctx).jobs.offset(jobID as isize)).dstBuff);

        /* Clear the job description, but keep the mutex/cond */
        ZSTD_memset(
            (*mtctx).jobs.offset(jobID as isize) as *mut u8,
            0,
            core::mem::size_of::<ZSTDMT_jobDescription>() as size_t,
        );
        (*(*mtctx).jobs.offset(jobID as isize)).job_mutex = mutex;
        (*(*mtctx).jobs.offset(jobID as isize)).job_cond = cond;
        jobID += 1;
    }
    (*mtctx).inBuff.buffer = g_nullBuffer;
    (*mtctx).inBuff.filled = 0;
    (*mtctx).allJobsCompleted = 1;
}

pub unsafe fn ZSTDMT_waitForAllJobsCompleted(mtctx: *mut ZSTDMT_CCtx) {
    while (*mtctx).doneJobID < (*mtctx).nextJobID {
        let jobID: c_uint = (*mtctx).doneJobID & (*mtctx).jobIDMask;
        ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*(*mtctx).jobs.offset(jobID as isize)).job_mutex);
        while (*(*mtctx).jobs.offset(jobID as isize)).consumed
            < (*(*mtctx).jobs.offset(jobID as isize)).src.size
        {
            ZSTD_pthread_cond_wait(
                &raw mut (*(*mtctx).jobs.offset(jobID as isize)).job_cond,
                &raw mut (*(*mtctx).jobs.offset(jobID as isize)).job_mutex,
            );
        }
        ZSTD_pthread_mutex_unlock(&raw mut (*(*mtctx).jobs.offset(jobID as isize)).job_mutex);
        (*mtctx).doneJobID += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_freeCCtx(mtctx_opaque: *mut OpaqueZSTDMT_CCtx) -> size_t {
    let mtctx: *mut ZSTDMT_CCtx = mtctx_opaque as *mut ZSTDMT_CCtx;
    if mtctx.is_null() {
        return 0; /* compatible with free on NULL */
    }
    if (*mtctx).providedFactory == 0 {
        POOL_free((*mtctx).factory); /* stop and free worker threads */
    }
    ZSTDMT_releaseAllJobResources(mtctx); /* release job resources into pools first */
    ZSTDMT_freeJobsTable((*mtctx).jobs, (*mtctx).jobIDMask + 1, (*mtctx).cMem);
    ZSTDMT_freeBufferPool((*mtctx).bufPool);
    ZSTDMT_freeCCtxPool((*mtctx).cctxPool);
    ZSTDMT_freeSeqPool((*mtctx).seqPool);
    ZSTDMT_serialState_free(&raw mut (*mtctx).serial);
    ZSTD_freeCDict((*mtctx).cdictLocal);
    if !(*mtctx).roundBuff.buffer.is_null() {
        ZSTD_customFree((*mtctx).roundBuff.buffer as *mut c_void, (*mtctx).cMem);
    }
    ZSTD_customFree(mtctx as *mut c_void, (*mtctx).cMem);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_sizeof_CCtx(mtctx_opaque: *mut OpaqueZSTDMT_CCtx) -> size_t {
    let mtctx: *mut ZSTDMT_CCtx = mtctx_opaque as *mut ZSTDMT_CCtx;
    if mtctx.is_null() {
        return 0; /* supports sizeof NULL */
    }
    (core::mem::size_of::<ZSTDMT_CCtx>() as size_t)
        .wrapping_add(POOL_sizeof((*mtctx).factory))
        .wrapping_add(ZSTDMT_sizeof_bufferPool((*mtctx).bufPool))
        .wrapping_add(
            (((*mtctx).jobIDMask + 1) as size_t)
                .wrapping_mul(core::mem::size_of::<ZSTDMT_jobDescription>() as size_t),
        )
        .wrapping_add(ZSTDMT_sizeof_CCtxPool((*mtctx).cctxPool))
        .wrapping_add(ZSTDMT_sizeof_seqPool((*mtctx).seqPool))
        .wrapping_add(ZSTD_sizeof_CDict((*mtctx).cdictLocal))
        .wrapping_add((*mtctx).roundBuff.capacity)
}

/* ZSTDMT_resize() : @return : error code if fails, 0 on success */
pub unsafe fn ZSTDMT_resize(mtctx: *mut ZSTDMT_CCtx, nbWorkers: c_uint) -> size_t {
    if POOL_resize((*mtctx).factory, nbWorkers as size_t) != 0 {
        return ERROR(ZSTD_error_memory_allocation);
    }
    {
        let _err = ZSTDMT_expandJobsTable(mtctx, nbWorkers);
        if ERR_isError(_err) != 0 {
            return _err;
        }
    }
    (*mtctx).bufPool = ZSTDMT_expandBufferPool((*mtctx).bufPool, BUF_POOL_MAX_NB_BUFFERS(nbWorkers));
    if (*mtctx).bufPool.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    (*mtctx).cctxPool = ZSTDMT_expandCCtxPool((*mtctx).cctxPool, nbWorkers as c_int);
    if (*mtctx).cctxPool.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    (*mtctx).seqPool = ZSTDMT_expandSeqPool((*mtctx).seqPool, nbWorkers);
    if (*mtctx).seqPool.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    ZSTDMT_CCtxParam_setNbWorkers(&raw mut (*mtctx).params, nbWorkers);
    0
}

/* ZSTDMT_updateCParams_whileCompressing() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_updateCParams_whileCompressing(
    mtctx_opaque: *mut OpaqueZSTDMT_CCtx,
    cctxParams: *const ZSTD_CCtx_params,
) {
    let mtctx: *mut ZSTDMT_CCtx = mtctx_opaque as *mut ZSTDMT_CCtx;
    let saved_wlog: U32 = (*mtctx).params.cParams.windowLog; /* Do not modify windowLog while compressing */
    let compressionLevel: c_int = (*cctxParams).compressionLevel;
    (*mtctx).params.compressionLevel = compressionLevel;
    {
        let mut cParams: ZSTD_compressionParameters = ZSTD_getCParamsFromCCtxParams(
            cctxParams,
            ZSTD_CONTENTSIZE_UNKNOWN,
            0,
            ZSTD_cpm_noAttachDict,
        );
        cParams.windowLog = saved_wlog;
        (*mtctx).params.cParams = cParams;
    }
}

/* ZSTDMT_getFrameProgression() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_getFrameProgression(
    mtctx_opaque: *mut OpaqueZSTDMT_CCtx,
) -> ZSTD_frameProgression {
    let mtctx: *mut ZSTDMT_CCtx = mtctx_opaque as *mut ZSTDMT_CCtx;
    let mut fps: ZSTD_frameProgression = core::mem::zeroed();
    fps.ingested = (*mtctx).consumed + (*mtctx).inBuff.filled as c_ulonglong;
    fps.consumed = (*mtctx).consumed;
    fps.produced = (*mtctx).produced;
    fps.flushed = (*mtctx).produced;
    fps.currentJobID = (*mtctx).nextJobID;
    fps.nbActiveWorkers = 0;
    {
        let mut jobNb: c_uint;
        let lastJobNb: c_uint = (*mtctx).nextJobID + (*mtctx).jobReady as c_uint;
        jobNb = (*mtctx).doneJobID;
        while jobNb < lastJobNb {
            let wJobID: c_uint = jobNb & (*mtctx).jobIDMask;
            let jobPtr: *mut ZSTDMT_jobDescription = (*mtctx).jobs.offset(wJobID as isize);
            ZSTD_pthread_mutex_lock(&raw mut (*jobPtr).job_mutex);
            {
                let cResult: size_t = (*jobPtr).cSize;
                let produced: size_t = if ZSTD_isError(cResult) != 0 { 0 } else { cResult };
                let flushed: size_t = if ZSTD_isError(cResult) != 0 {
                    0
                } else {
                    (*jobPtr).dstFlushed
                };
                fps.ingested += (*jobPtr).src.size as c_ulonglong;
                fps.consumed += (*jobPtr).consumed as c_ulonglong;
                fps.produced += produced as c_ulonglong;
                fps.flushed += flushed as c_ulonglong;
                fps.nbActiveWorkers += ((*jobPtr).consumed < (*jobPtr).src.size) as c_uint;
            }
            ZSTD_pthread_mutex_unlock(&raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_mutex);
            jobNb += 1;
        }
    }
    fps
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_toFlushNow(mtctx_opaque: *mut OpaqueZSTDMT_CCtx) -> size_t {
    let mtctx: *mut ZSTDMT_CCtx = mtctx_opaque as *mut ZSTDMT_CCtx;
    let toFlush: size_t;
    let jobID: c_uint = (*mtctx).doneJobID;
    if jobID == (*mtctx).nextJobID {
        return 0; /* no active job => nothing to flush */
    }

    /* look into oldest non-fully-flushed job */
    {
        let wJobID: c_uint = jobID & (*mtctx).jobIDMask;
        let jobPtr: *mut ZSTDMT_jobDescription = (*mtctx).jobs.offset(wJobID as isize);
        ZSTD_pthread_mutex_lock(&raw mut (*jobPtr).job_mutex);
        {
            let cResult: size_t = (*jobPtr).cSize;
            let produced: size_t = if ZSTD_isError(cResult) != 0 { 0 } else { cResult };
            let flushed: size_t = if ZSTD_isError(cResult) != 0 {
                0
            } else {
                (*jobPtr).dstFlushed
            };
            toFlush = produced - flushed;
            /* if toFlush==0, nothing is available to flush. */
        }
        ZSTD_pthread_mutex_unlock(&raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_mutex);
    }

    toFlush
}

/* ------------------------------------------ */
/* =====   Multi-threaded compression   ===== */
/* ------------------------------------------ */

pub unsafe fn ZSTDMT_computeTargetJobLog(params: *const ZSTD_CCtx_params) -> c_uint {
    let jobLog: c_uint;
    if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
        jobLog = MAX(
            21,
            (ZSTD_cycleLog((*params).cParams.chainLog, (*params).cParams.strategy) + 3) as size_t,
        ) as c_uint;
    } else {
        jobLog = MAX(20, ((*params).cParams.windowLog + 2) as size_t) as c_uint;
    }
    MIN(jobLog as size_t, ZSTDMT_JOBLOG_MAX() as size_t) as c_uint
}

pub unsafe fn ZSTDMT_overlapLog_default(strat: ZSTD_strategy) -> c_int {
    /* switch(strat) */
    if strat == ZSTD_btultra2 {
        return 9;
    }
    if strat == ZSTD_btultra || strat == ZSTD_btopt {
        return 8;
    }
    if strat == ZSTD_btlazy2 || strat == ZSTD_lazy2 {
        return 7;
    }
    /* ZSTD_lazy, ZSTD_greedy, ZSTD_dfast, ZSTD_fast, default */
    6
}

pub unsafe fn ZSTDMT_overlapLog(ovlog: c_int, strat: ZSTD_strategy) -> c_int {
    if ovlog == 0 {
        return ZSTDMT_overlapLog_default(strat);
    }
    ovlog
}

pub unsafe fn ZSTDMT_computeOverlapSize(params: *const ZSTD_CCtx_params) -> size_t {
    let overlapRLog: c_int = 9 - ZSTDMT_overlapLog((*params).overlapLog, (*params).cParams.strategy);
    let mut ovLog: c_int = if overlapRLog >= 8 {
        0
    } else {
        (*params).cParams.windowLog as c_int - overlapRLog
    };
    if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
        ovLog = MIN(
            (*params).cParams.windowLog as size_t,
            (ZSTDMT_computeTargetJobLog(params) - 2) as size_t,
        ) as c_int
            - overlapRLog;
    }
    if ovLog == 0 {
        0
    } else {
        1usize << ovLog
    }
}

/* ====================================== */
/* =======      Streaming API     ======= */
/* ====================================== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_initCStream_internal(
    mtctx_opaque: *mut OpaqueZSTDMT_CCtx,
    dict: *const c_void,
    dictSize: size_t,
    dictContentType: ZSTD_dictContentType_e,
    cdict: *const ZSTD_CDict,
    mut params: ZSTD_CCtx_params,
    pledgedSrcSize: c_ulonglong,
) -> size_t {
    let mtctx: *mut ZSTDMT_CCtx = mtctx_opaque as *mut ZSTDMT_CCtx;

    /* init */
    if params.nbWorkers != (*mtctx).params.nbWorkers {
        let _err = ZSTDMT_resize(mtctx, params.nbWorkers as c_uint);
        if ERR_isError(_err) != 0 {
            return _err;
        }
    }

    if params.jobSize != 0 && params.jobSize < ZSTDMT_JOBSIZE_MIN {
        params.jobSize = ZSTDMT_JOBSIZE_MIN;
    }
    if params.jobSize > ZSTDMT_JOBSIZE_MAX() {
        params.jobSize = ZSTDMT_JOBSIZE_MAX();
    }

    if (*mtctx).allJobsCompleted == 0 {
        /* previous compression not correctly finished */
        ZSTDMT_waitForAllJobsCompleted(mtctx);
        ZSTDMT_releaseAllJobResources(mtctx);
        (*mtctx).allJobsCompleted = 1;
    }

    (*mtctx).params = core::ptr::read(&raw const params);
    (*mtctx).frameContentSize = pledgedSrcSize;
    ZSTD_freeCDict((*mtctx).cdictLocal);
    if !dict.is_null() {
        (*mtctx).cdictLocal = ZSTD_createCDict_advanced(
            dict,
            dictSize,
            ZSTD_dlm_byCopy,
            dictContentType, /* note : a loadPrefix becomes an internal CDict */
            params.cParams,
            (*mtctx).cMem,
        );
        (*mtctx).cdict = (*mtctx).cdictLocal;
        if (*mtctx).cdictLocal.is_null() {
            return ERROR(ZSTD_error_memory_allocation);
        }
    } else {
        (*mtctx).cdictLocal = null_mut();
        (*mtctx).cdict = cdict;
    }

    (*mtctx).targetPrefixSize = ZSTDMT_computeOverlapSize(&raw const params);
    (*mtctx).targetSectionSize = params.jobSize;
    if (*mtctx).targetSectionSize == 0 {
        (*mtctx).targetSectionSize = 1usize << ZSTDMT_computeTargetJobLog(&raw const params);
    }

    if params.rsyncable != 0 {
        /* Aim for the targetsectionSize as the average job size. */
        let jobSizeKB: U32 = ((*mtctx).targetSectionSize >> 10) as U32;
        let rsyncBits: U32 = ZSTD_highbit32(jobSizeKB) + 10;
        (*mtctx).rsync.hash = 0;
        (*mtctx).rsync.hitMask = (1u64 << rsyncBits) - 1;
        (*mtctx).rsync.primePower = ZSTD_rollingHash_primePower(RSYNC_LENGTH as U32);
    }
    if (*mtctx).targetSectionSize < (*mtctx).targetPrefixSize {
        (*mtctx).targetSectionSize = (*mtctx).targetPrefixSize; /* job size must be >= overlap size */
    }
    ZSTDMT_setBufferSize((*mtctx).bufPool, ZSTD_compressBound((*mtctx).targetSectionSize));
    {
        /* If ldm is enabled we need windowSize space. */
        let windowSize: size_t = if (*mtctx).params.ldmParams.enableLdm == ZSTD_ps_enable {
            (1usize << (*mtctx).params.cParams.windowLog)
        } else {
            0
        };
        let nbSlackBuffers: size_t = 2 + ((*mtctx).targetPrefixSize > 0) as size_t;
        let slackSize: size_t = (*mtctx).targetSectionSize.wrapping_mul(nbSlackBuffers);
        let nbWorkers: size_t = MAX((*mtctx).params.nbWorkers as size_t, 1);
        let sectionsSize: size_t = (*mtctx).targetSectionSize.wrapping_mul(nbWorkers);
        let capacity: size_t = MAX(windowSize, sectionsSize).wrapping_add(slackSize);
        if (*mtctx).roundBuff.capacity < capacity {
            if !(*mtctx).roundBuff.buffer.is_null() {
                ZSTD_customFree((*mtctx).roundBuff.buffer as *mut c_void, (*mtctx).cMem);
            }
            (*mtctx).roundBuff.buffer = ZSTD_customMalloc(capacity, (*mtctx).cMem) as *mut BYTE;
            if (*mtctx).roundBuff.buffer.is_null() {
                (*mtctx).roundBuff.capacity = 0;
                return ERROR(ZSTD_error_memory_allocation);
            }
            (*mtctx).roundBuff.capacity = capacity;
        }
    }
    (*mtctx).roundBuff.pos = 0;
    (*mtctx).inBuff.buffer = g_nullBuffer;
    (*mtctx).inBuff.filled = 0;
    (*mtctx).inBuff.prefix = kNullRange;
    (*mtctx).doneJobID = 0;
    (*mtctx).nextJobID = 0;
    (*mtctx).frameEnded = 0;
    (*mtctx).allJobsCompleted = 0;
    (*mtctx).consumed = 0;
    (*mtctx).produced = 0;

    /* update dictionary */
    ZSTD_freeCDict((*mtctx).cdictLocal);
    (*mtctx).cdictLocal = null_mut();
    (*mtctx).cdict = null_mut();
    if !dict.is_null() {
        if dictContentType == ZSTD_dct_rawContent {
            (*mtctx).inBuff.prefix.start = dict as *const BYTE as *const c_void;
            (*mtctx).inBuff.prefix.size = dictSize;
        } else {
            /* note : a loadPrefix becomes an internal CDict */
            (*mtctx).cdictLocal = ZSTD_createCDict_advanced(
                dict,
                dictSize,
                ZSTD_dlm_byRef,
                dictContentType,
                params.cParams,
                (*mtctx).cMem,
            );
            (*mtctx).cdict = (*mtctx).cdictLocal;
            if (*mtctx).cdictLocal.is_null() {
                return ERROR(ZSTD_error_memory_allocation);
            }
        }
    } else {
        (*mtctx).cdict = cdict;
    }

    if ZSTDMT_serialState_reset(
        &raw mut (*mtctx).serial,
        (*mtctx).seqPool,
        core::ptr::read(&raw const params),
        (*mtctx).targetSectionSize,
        dict,
        dictSize,
        dictContentType,
    ) != 0
    {
        return ERROR(ZSTD_error_memory_allocation);
    }

    0
}

/* ZSTDMT_writeLastEmptyBlock() */
pub unsafe fn ZSTDMT_writeLastEmptyBlock(job: *mut ZSTDMT_jobDescription) {
    (*job).dstBuff = ZSTDMT_getBuffer((*job).bufPool);
    if (*job).dstBuff.start.is_null() {
        (*job).cSize = ERROR(ZSTD_error_memory_allocation);
        return;
    }
    (*job).src = kNullRange;
    (*job).cSize = ZSTD_writeLastEmptyBlock((*job).dstBuff.start, (*job).dstBuff.capacity);
}

pub unsafe fn ZSTDMT_createCompressionJob(
    mtctx: *mut ZSTDMT_CCtx,
    srcSize: size_t,
    endOp: ZSTD_EndDirective,
) -> size_t {
    let jobID: c_uint = (*mtctx).nextJobID & (*mtctx).jobIDMask;
    let endFrame: c_int = (endOp == ZSTD_e_end) as c_int;

    if (*mtctx).nextJobID > (*mtctx).doneJobID + (*mtctx).jobIDMask {
        return 0;
    }

    if (*mtctx).jobReady == 0 {
        let src: *const BYTE = (*mtctx).inBuff.buffer.start as *const BYTE;
        (*(*mtctx).jobs.offset(jobID as isize)).src.start = src as *const c_void;
        (*(*mtctx).jobs.offset(jobID as isize)).src.size = srcSize;
        (*(*mtctx).jobs.offset(jobID as isize)).prefix = (*mtctx).inBuff.prefix;
        (*(*mtctx).jobs.offset(jobID as isize)).consumed = 0;
        (*(*mtctx).jobs.offset(jobID as isize)).cSize = 0;
        (*(*mtctx).jobs.offset(jobID as isize)).params = core::ptr::read(&raw const (*mtctx).params);
        (*(*mtctx).jobs.offset(jobID as isize)).cdict =
            if (*mtctx).nextJobID == 0 { (*mtctx).cdict } else { null_mut() };
        (*(*mtctx).jobs.offset(jobID as isize)).fullFrameSize = (*mtctx).frameContentSize;
        (*(*mtctx).jobs.offset(jobID as isize)).dstBuff = g_nullBuffer;
        (*(*mtctx).jobs.offset(jobID as isize)).cctxPool = (*mtctx).cctxPool;
        (*(*mtctx).jobs.offset(jobID as isize)).bufPool = (*mtctx).bufPool;
        (*(*mtctx).jobs.offset(jobID as isize)).seqPool = (*mtctx).seqPool;
        (*(*mtctx).jobs.offset(jobID as isize)).serial = &raw mut (*mtctx).serial;
        (*(*mtctx).jobs.offset(jobID as isize)).jobID = (*mtctx).nextJobID;
        (*(*mtctx).jobs.offset(jobID as isize)).firstJob = ((*mtctx).nextJobID == 0) as c_uint;
        (*(*mtctx).jobs.offset(jobID as isize)).lastJob = endFrame as c_uint;
        (*(*mtctx).jobs.offset(jobID as isize)).frameChecksumNeeded =
            ((*mtctx).params.fParams.checksumFlag != 0 && endFrame != 0 && (*mtctx).nextJobID > 0)
                as c_uint;
        (*(*mtctx).jobs.offset(jobID as isize)).dstFlushed = 0;

        /* Update the round buffer pos and clear the input buffer to be reset */
        (*mtctx).roundBuff.pos = (*mtctx).roundBuff.pos.wrapping_add(srcSize);
        (*mtctx).inBuff.buffer = g_nullBuffer;
        (*mtctx).inBuff.filled = 0;
        /* Set the prefix for next job */
        if endFrame == 0 {
            let newPrefixSize: size_t = MIN(srcSize, (*mtctx).targetPrefixSize);
            (*mtctx).inBuff.prefix.start =
                (src.wrapping_add(srcSize).wrapping_sub(newPrefixSize)) as *const c_void;
            (*mtctx).inBuff.prefix.size = newPrefixSize;
        } else {
            /* endFrame==1 => no need for another input buffer */
            (*mtctx).inBuff.prefix = kNullRange;
            (*mtctx).frameEnded = endFrame as c_uint;
            if (*mtctx).nextJobID == 0 {
                /* single job exception : checksum is already calculated directly within worker thread */
                (*mtctx).params.fParams.checksumFlag = 0;
            }
        }

        if (srcSize == 0) && ((*mtctx).nextJobID > 0)
        /*single job must also write frame header*/
        {
            ZSTDMT_writeLastEmptyBlock((*mtctx).jobs.offset(jobID as isize));
            (*mtctx).nextJobID += 1;
            return 0;
        }
    }

    if POOL_tryAdd(
        (*mtctx).factory,
        ZSTDMT_compressionJob,
        (*mtctx).jobs.offset(jobID as isize) as *mut c_void,
    ) != 0
    {
        (*mtctx).nextJobID += 1;
        (*mtctx).jobReady = 0;
    } else {
        (*mtctx).jobReady = 1;
    }
    0
}

/* ZSTDMT_flushProduced() */
pub unsafe fn ZSTDMT_flushProduced(
    mtctx: *mut ZSTDMT_CCtx,
    output: *mut ZSTD_outBuffer,
    blockToFlush: c_uint,
    end: ZSTD_EndDirective,
) -> size_t {
    let wJobID: c_uint = (*mtctx).doneJobID & (*mtctx).jobIDMask;

    ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_mutex);
    if blockToFlush != 0 && ((*mtctx).doneJobID < (*mtctx).nextJobID) {
        while (*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed
            == (*(*mtctx).jobs.offset(wJobID as isize)).cSize
        {
            /* nothing to flush */
            if (*(*mtctx).jobs.offset(wJobID as isize)).consumed
                == (*(*mtctx).jobs.offset(wJobID as isize)).src.size
            {
                break;
            }
            ZSTD_pthread_cond_wait(
                &raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_cond,
                &raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_mutex,
            ); /* block when nothing to flush but some to come */
        }
    }

    /* try to flush something */
    {
        let mut cSize: size_t = (*(*mtctx).jobs.offset(wJobID as isize)).cSize; /* shared */
        let srcConsumed: size_t = (*(*mtctx).jobs.offset(wJobID as isize)).consumed; /* shared */
        let srcSize: size_t = (*(*mtctx).jobs.offset(wJobID as isize)).src.size; /* read-only */
        ZSTD_pthread_mutex_unlock(&raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_mutex);
        if ZSTD_isError(cSize) != 0 {
            ZSTDMT_waitForAllJobsCompleted(mtctx);
            ZSTDMT_releaseAllJobResources(mtctx);
            return cSize;
        }
        /* add frame checksum if necessary (can only happen once) */
        if (srcConsumed == srcSize) /* job completed -> worker no longer active */
            && (*(*mtctx).jobs.offset(wJobID as isize)).frameChecksumNeeded != 0
        {
            let checksum: U32 = ZSTD_XXH64_digest(&raw const (*mtctx).serial.xxhState) as U32;
            MEM_writeLE32(
                ((*(*mtctx).jobs.offset(wJobID as isize)).dstBuff.start as *mut c_char)
                    .wrapping_add((*(*mtctx).jobs.offset(wJobID as isize)).cSize) as *mut u8,
                checksum,
            );
            cSize += 4;
            (*(*mtctx).jobs.offset(wJobID as isize)).cSize += 4; /* can write this shared value, as worker is no longer active */
            (*(*mtctx).jobs.offset(wJobID as isize)).frameChecksumNeeded = 0;
        }

        if cSize > 0 {
            /* compression is ongoing or completed */
            let toFlush: size_t = MIN(
                cSize - (*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed,
                (*output).size - (*output).pos,
            );
            if toFlush > 0 {
                ZSTD_memcpy(
                    ((*output).dst as *mut c_char).wrapping_add((*output).pos) as *mut u8,
                    ((*(*mtctx).jobs.offset(wJobID as isize)).dstBuff.start as *const c_char)
                        .wrapping_add((*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed)
                        as *const u8,
                    toFlush,
                );
            }
            (*output).pos = (*output).pos.wrapping_add(toFlush);
            (*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed =
                (*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed.wrapping_add(toFlush); /* can write : this value is only used by mtctx */

            if (srcConsumed == srcSize) /* job is completed */
                && ((*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed == cSize)
            {
                /* output buffer fully flushed => free this job position */
                ZSTDMT_releaseBuffer(
                    (*mtctx).bufPool,
                    (*(*mtctx).jobs.offset(wJobID as isize)).dstBuff,
                );
                (*(*mtctx).jobs.offset(wJobID as isize)).dstBuff = g_nullBuffer;
                (*(*mtctx).jobs.offset(wJobID as isize)).cSize = 0; /* ensure this job slot is considered "not started" in future check */
                (*mtctx).consumed = (*mtctx).consumed + srcSize as c_ulonglong;
                (*mtctx).produced = (*mtctx).produced + cSize as c_ulonglong;
                (*mtctx).doneJobID += 1;
            }
        }

        /* return value : how many bytes left in buffer ; fake it to 1 when unknown but >0 */
        if cSize > (*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed {
            return cSize - (*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed;
        }
        if srcSize > srcConsumed {
            return 1; /* current job not completely compressed */
        }
    }
    if (*mtctx).doneJobID < (*mtctx).nextJobID {
        return 1; /* some more jobs ongoing */
    }
    if (*mtctx).jobReady != 0 {
        return 1; /* one job is ready to push, just not yet in the list */
    }
    if (*mtctx).inBuff.filled > 0 {
        return 1; /* input is not empty, and still needs to be converted into a job */
    }
    (*mtctx).allJobsCompleted = (*mtctx).frameEnded; /* all jobs are entirely flushed => if this one is last one, frame is completed */
    if end == ZSTD_e_end {
        return ((*mtctx).frameEnded == 0) as size_t; /* for ZSTD_e_end, question becomes : is frame completed ? */
    }
    0 /* internal buffers fully flushed */
}

pub unsafe fn ZSTDMT_getInputDataInUse(mtctx: *mut ZSTDMT_CCtx) -> Range {
    let firstJobID: c_uint = (*mtctx).doneJobID;
    let lastJobID: c_uint = (*mtctx).nextJobID;
    let mut jobID: c_uint;

    /* no need to check during first round */
    let roundBuffCapacity: size_t = (*mtctx).roundBuff.capacity;
    let nbJobs1stRoundMin: size_t = roundBuffCapacity / (*mtctx).targetSectionSize;
    if (lastJobID as size_t) < nbJobs1stRoundMin {
        return kNullRange;
    }

    jobID = firstJobID;
    while jobID < lastJobID {
        let wJobID: c_uint = jobID & (*mtctx).jobIDMask;
        let consumed: size_t;

        ZSTD_PTHREAD_MUTEX_LOCK(&raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_mutex);
        consumed = (*(*mtctx).jobs.offset(wJobID as isize)).consumed;
        ZSTD_pthread_mutex_unlock(&raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_mutex);

        if consumed < (*(*mtctx).jobs.offset(wJobID as isize)).src.size {
            let mut range: Range = (*(*mtctx).jobs.offset(wJobID as isize)).prefix;
            if range.size == 0 {
                /* Empty prefix */
                range = (*(*mtctx).jobs.offset(wJobID as isize)).src;
            }
            return range;
        }
        jobID += 1;
    }
    kNullRange
}

/* Returns non-zero iff buffer and range overlap. */
pub unsafe fn ZSTDMT_isOverlapped(buffer: Buffer, range: Range) -> c_int {
    let bufferStart: *const BYTE = buffer.start as *const BYTE;
    let rangeStart: *const BYTE = range.start as *const BYTE;

    if rangeStart.is_null() || bufferStart.is_null() {
        return 0;
    }

    {
        let bufferEnd: *const BYTE = bufferStart.wrapping_add(buffer.capacity);
        let rangeEnd: *const BYTE = rangeStart.wrapping_add(range.size);

        /* Empty ranges cannot overlap */
        if bufferStart == bufferEnd || rangeStart == rangeEnd {
            return 0;
        }

        (bufferStart < rangeEnd && rangeStart < bufferEnd) as c_int
    }
}

pub unsafe fn ZSTDMT_doesOverlapWindow(buffer: Buffer, window: ZSTD_window_t) -> c_int {
    let mut extDict: Range = Range {
        start: null_mut(),
        size: 0,
    };
    let mut prefix: Range = Range {
        start: null_mut(),
        size: 0,
    };

    extDict.start = window.dictBase.wrapping_add(window.lowLimit as usize) as *const c_void;
    extDict.size = (window.dictLimit - window.lowLimit) as size_t;

    prefix.start = window.base.wrapping_add(window.dictLimit as usize) as *const c_void;
    prefix.size = window
        .nextSrc
        .offset_from(window.base.wrapping_add(window.dictLimit as usize)) as size_t;

    (ZSTDMT_isOverlapped(buffer, extDict) != 0 || ZSTDMT_isOverlapped(buffer, prefix) != 0) as c_int
}

pub unsafe fn ZSTDMT_waitForLdmComplete(mtctx: *mut ZSTDMT_CCtx, buffer: Buffer) {
    if (*mtctx).params.ldmParams.enableLdm == ZSTD_ps_enable {
        let mutex: *mut ZSTD_pthread_mutex_t = &raw mut (*mtctx).serial.ldmWindowMutex;
        ZSTD_PTHREAD_MUTEX_LOCK(mutex);
        while ZSTDMT_doesOverlapWindow(buffer, (*mtctx).serial.ldmWindow) != 0 {
            ZSTD_pthread_cond_wait(&raw mut (*mtctx).serial.ldmWindowCond, mutex);
        }
        ZSTD_pthread_mutex_unlock(mutex);
    }
}

pub unsafe fn ZSTDMT_tryGetInputRange(mtctx: *mut ZSTDMT_CCtx) -> c_int {
    let inUse: Range = ZSTDMT_getInputDataInUse(mtctx);
    let spaceLeft: size_t = (*mtctx).roundBuff.capacity - (*mtctx).roundBuff.pos;
    let spaceNeeded: size_t = (*mtctx).targetSectionSize;
    let mut buffer: Buffer = Buffer {
        start: null_mut(),
        capacity: 0,
    };

    if spaceLeft < spaceNeeded {
        /* ZSTD_invalidateRepCodes() doesn't work for extDict variants.
         * Simply copy the prefix to the beginning in that case. */
        let start: *mut BYTE = (*mtctx).roundBuff.buffer;
        let prefixSize: size_t = (*mtctx).inBuff.prefix.size;

        buffer.start = start as *mut c_void;
        buffer.capacity = prefixSize;
        if ZSTDMT_isOverlapped(buffer, inUse) != 0 {
            return 0;
        }
        ZSTDMT_waitForLdmComplete(mtctx, buffer);
        ZSTD_memmove(start, (*mtctx).inBuff.prefix.start as *const u8, prefixSize);
        (*mtctx).inBuff.prefix.start = start as *const c_void;
        (*mtctx).roundBuff.pos = prefixSize;
    }
    buffer.start = (*mtctx).roundBuff.buffer.wrapping_add((*mtctx).roundBuff.pos) as *mut c_void;
    buffer.capacity = spaceNeeded;

    if ZSTDMT_isOverlapped(buffer, inUse) != 0 {
        return 0;
    }

    ZSTDMT_waitForLdmComplete(mtctx, buffer);

    (*mtctx).inBuff.buffer = buffer;
    (*mtctx).inBuff.filled = 0;
    1
}

#[repr(C)]
pub struct SyncPoint {
    pub toLoad: size_t, /* The number of bytes to load from the input. */
    pub flush: c_int,   /* Boolean declaring if we must flush because we found a synchronization point. */
}

pub unsafe fn findSynchronizationPoint(mtctx: *const ZSTDMT_CCtx, input: ZSTD_inBuffer) -> SyncPoint {
    let istart: *const BYTE = (input.src as *const BYTE).wrapping_add(input.pos);
    let primePower: U64 = (*mtctx).rsync.primePower;
    let hitMask: U64 = (*mtctx).rsync.hitMask;

    let mut syncPoint: SyncPoint = SyncPoint {
        toLoad: 0,
        flush: 0,
    };
    let mut hash: U64;
    let prev: *const BYTE;
    let mut pos: size_t;

    syncPoint.toLoad = MIN(
        input.size - input.pos,
        (*mtctx).targetSectionSize - (*mtctx).inBuff.filled,
    );
    syncPoint.flush = 0;
    if (*mtctx).params.rsyncable == 0 {
        /* Rsync is disabled. */
        return syncPoint;
    }
    if (*mtctx).inBuff.filled + input.size - input.pos < RSYNC_MIN_BLOCK_SIZE {
        return syncPoint;
    }
    if (*mtctx).inBuff.filled + syncPoint.toLoad < RSYNC_LENGTH {
        return syncPoint;
    }
    /* Initialize the loop variables. */
    if (*mtctx).inBuff.filled < RSYNC_MIN_BLOCK_SIZE {
        pos = RSYNC_MIN_BLOCK_SIZE - (*mtctx).inBuff.filled;
        if pos >= RSYNC_LENGTH {
            prev = istart.wrapping_add(pos).wrapping_sub(RSYNC_LENGTH);
            hash = ZSTD_rollingHash_compute(prev as *const c_void, RSYNC_LENGTH);
        } else {
            prev = ((*mtctx).inBuff.buffer.start as *const BYTE)
                .wrapping_add((*mtctx).inBuff.filled)
                .wrapping_sub(RSYNC_LENGTH);
            hash = ZSTD_rollingHash_compute(
                prev.wrapping_add(pos) as *const c_void,
                RSYNC_LENGTH - pos,
            );
            hash = ZSTD_rollingHash_append(hash, istart as *const c_void, pos);
        }
    } else {
        pos = 0;
        prev = ((*mtctx).inBuff.buffer.start as *const BYTE)
            .wrapping_add((*mtctx).inBuff.filled)
            .wrapping_sub(RSYNC_LENGTH);
        hash = ZSTD_rollingHash_compute(prev as *const c_void, RSYNC_LENGTH);
        if (hash & hitMask) == hitMask {
            syncPoint.toLoad = 0;
            syncPoint.flush = 1;
            return syncPoint;
        }
    }
    while pos < syncPoint.toLoad {
        let toRemove: BYTE = if pos < RSYNC_LENGTH {
            *prev.wrapping_add(pos)
        } else {
            *istart.wrapping_add(pos - RSYNC_LENGTH)
        };
        hash = ZSTD_rollingHash_rotate(hash, toRemove, *istart.wrapping_add(pos), primePower);
        if (hash & hitMask) == hitMask {
            syncPoint.toLoad = pos + 1;
            syncPoint.flush = 1;
            pos += 1; /* for assert */
            break;
        }
        pos += 1;
    }
    syncPoint
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_nextInputSizeHint(mtctx_opaque: *const OpaqueZSTDMT_CCtx) -> size_t {
    let mtctx: *const ZSTDMT_CCtx = mtctx_opaque as *const ZSTDMT_CCtx;
    let mut hintInSize: size_t = (*mtctx).targetSectionSize - (*mtctx).inBuff.filled;
    if hintInSize == 0 {
        hintInSize = (*mtctx).targetSectionSize;
    }
    hintInSize
}

/** ZSTDMT_compressStream_generic() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_compressStream_generic(
    mtctx_opaque: *mut OpaqueZSTDMT_CCtx,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
    mut endOp: ZSTD_EndDirective,
) -> size_t {
    let mtctx: *mut ZSTDMT_CCtx = mtctx_opaque as *mut ZSTDMT_CCtx;
    let mut forwardInputProgress: c_uint = 0;

    if ((*mtctx).frameEnded != 0) && (endOp == ZSTD_e_continue) {
        /* current frame being ended. Only flush/end are allowed */
        return ERROR(ZSTD_error_stage_wrong);
    }

    /* fill input buffer */
    if ((*mtctx).jobReady == 0) && ((*input).size > (*input).pos) {
        /* support NULL input */
        if (*mtctx).inBuff.buffer.start.is_null() {
            if ZSTDMT_tryGetInputRange(mtctx) == 0 {
                /* It is only possible for this operation to fail if there are
                 * still compression jobs ongoing. */
            }
        }
        if !(*mtctx).inBuff.buffer.start.is_null() {
            let syncPoint: SyncPoint = findSynchronizationPoint(mtctx, core::ptr::read(input));
            if syncPoint.flush != 0 && endOp == ZSTD_e_continue {
                endOp = ZSTD_e_flush;
            }
            ZSTD_memcpy(
                ((*mtctx).inBuff.buffer.start as *mut c_char).wrapping_add((*mtctx).inBuff.filled)
                    as *mut u8,
                ((*input).src as *const c_char).wrapping_add((*input).pos) as *const u8,
                syncPoint.toLoad,
            );
            (*input).pos = (*input).pos.wrapping_add(syncPoint.toLoad);
            (*mtctx).inBuff.filled = (*mtctx).inBuff.filled.wrapping_add(syncPoint.toLoad);
            forwardInputProgress = (syncPoint.toLoad > 0) as c_uint;
        }
    }
    if ((*input).pos < (*input).size) && (endOp == ZSTD_e_end) {
        /* Can't end yet because the input is not fully consumed. */
        endOp = ZSTD_e_flush;
    }

    if ((*mtctx).jobReady != 0)
        || ((*mtctx).inBuff.filled >= (*mtctx).targetSectionSize) /* filled enough : let's compress */
        || ((endOp != ZSTD_e_continue) && ((*mtctx).inBuff.filled > 0)) /* something to flush : let's go */
        || ((endOp == ZSTD_e_end) && ((*mtctx).frameEnded == 0))
    {
        /* must finish the frame with a zero-size block */
        let jobSize: size_t = (*mtctx).inBuff.filled;
        let _err = ZSTDMT_createCompressionJob(mtctx, jobSize, endOp);
        if ERR_isError(_err) != 0 {
            return _err;
        }
    }

    /* check for potential compressed data ready to be flushed */
    {
        let remainingToFlush: size_t =
            ZSTDMT_flushProduced(mtctx, output, (forwardInputProgress == 0) as c_uint, endOp); /* block if there was no forward input progress */
        if (*input).pos < (*input).size {
            return MAX(remainingToFlush, 1); /* input not consumed : do not end flush yet */
        }
        remainingToFlush
    }
}
