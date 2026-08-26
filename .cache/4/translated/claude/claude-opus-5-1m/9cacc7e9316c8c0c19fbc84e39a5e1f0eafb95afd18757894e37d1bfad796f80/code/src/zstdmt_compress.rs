//! Translation of compress/zstdmt_compress.c (+ zstdmt_compress.h)
//!
//! NOTE on the preprocessor: in this version of zstd only ONE construct in
//! `zstdmt_compress.c` is guarded by `ZSTD_MULTITHREAD` — the body of
//! `ZSTDMT_createCCtx_advanced()`.  Everything else in the file is compiled
//! unconditionally; the multi-threading primitives simply degrade to the
//! no-op macros from `common/threading.h`'s
//! `#else /* ZSTD_MULTITHREAD not defined */` block
//! (`ZSTD_pthread_mutex_t`/`ZSTD_pthread_cond_t` become `int`, and every
//! lock/unlock/wait/signal/broadcast becomes `((void)(a))`).
//! Those no-op inline helpers are reproduced below.
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

use crate::bits::*;
use crate::error_private::*;
use crate::mem::*;
use crate::pool::*;
use crate::xxhash::*;
use crate::zstd_compress_internal::*;
use crate::zstd_h::*;
use crate::zstd_internal::*;

/* ======   threading.h, ZSTD_MULTITHREAD *not* defined   ====== */

pub type ZSTD_pthread_mutex_t = core::ffi::c_int;
pub type ZSTD_pthread_cond_t = core::ffi::c_int;

#[inline(always)]
pub unsafe fn ZSTD_pthread_mutex_init(
    _a: *mut ZSTD_pthread_mutex_t,
    _b: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    0
}
#[inline(always)]
pub unsafe fn ZSTD_pthread_mutex_destroy(_a: *mut ZSTD_pthread_mutex_t) {}
#[inline(always)]
pub unsafe fn ZSTD_pthread_mutex_lock(_a: *mut ZSTD_pthread_mutex_t) {}
#[inline(always)]
pub unsafe fn ZSTD_pthread_mutex_unlock(_a: *mut ZSTD_pthread_mutex_t) {}
#[inline(always)]
pub unsafe fn ZSTD_pthread_cond_init(
    _a: *mut ZSTD_pthread_cond_t,
    _b: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    0
}
#[inline(always)]
pub unsafe fn ZSTD_pthread_cond_destroy(_a: *mut ZSTD_pthread_cond_t) {}
#[inline(always)]
pub unsafe fn ZSTD_pthread_cond_wait(_a: *mut ZSTD_pthread_cond_t, _b: *mut ZSTD_pthread_mutex_t) {}
#[inline(always)]
pub unsafe fn ZSTD_pthread_cond_signal(_a: *mut ZSTD_pthread_cond_t) {}
#[inline(always)]
pub unsafe fn ZSTD_pthread_cond_broadcast(_a: *mut ZSTD_pthread_cond_t) {}

/* `#define ZSTD_PTHREAD_MUTEX_LOCK(m) ZSTD_pthread_mutex_lock(m)` (DEBUGLEVEL == 0) */
#[inline(always)]
pub unsafe fn ZSTD_PTHREAD_MUTEX_LOCK(m: *mut ZSTD_pthread_mutex_t) {
    ZSTD_pthread_mutex_lock(m);
}

/* ======   zstdmt_compress.h : constants   ====== */

/// `#define ZSTDMT_NBWORKERS_MAX ((sizeof(void*)==4) ? 64 : 256)`
pub const ZSTDMT_NBWORKERS_MAX: core::ffi::c_uint =
    if core::mem::size_of::<*mut core::ffi::c_void>() == 4 {
        64
    } else {
        256
    };
/// `#define ZSTDMT_JOBSIZE_MIN (512 KB)`
pub const ZSTDMT_JOBSIZE_MIN: usize = 512 * (1 << 10);
/// `#define ZSTDMT_JOBLOG_MAX (MEM_32bits() ? 29 : 30)`
pub const ZSTDMT_JOBLOG_MAX: core::ffi::c_int = if core::mem::size_of::<usize>() == 4 {
    29
} else {
    30
};
/// `#define ZSTDMT_JOBSIZE_MAX (MEM_32bits() ? (512 MB) : (1024 MB))`
pub const ZSTDMT_JOBSIZE_MAX: core::ffi::c_int = if core::mem::size_of::<usize>() == 4 {
    512 * (1 << 20)
} else {
    1024 * (1 << 20)
};

/// `#define ZSTD_RESIZE_SEQPOOL 0`
pub const ZSTD_RESIZE_SEQPOOL: core::ffi::c_int = 0;

/* =====   Buffer Pool   ===== */
/* a single Buffer Pool can be invoked from multiple threads in parallel */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Buffer {
    pub start: *mut core::ffi::c_void,
    pub capacity: usize,
}

pub const g_nullBuffer: Buffer = Buffer {
    start: core::ptr::null_mut(),
    capacity: 0,
};

#[repr(C)]
pub struct ZSTDMT_bufferPool {
    pub poolMutex: ZSTD_pthread_mutex_t,
    pub bufferSize: usize,
    pub totalBuffers: core::ffi::c_uint,
    pub nbBuffers: core::ffi::c_uint,
    pub cMem: ZSTD_customMem,
    pub buffers: *mut Buffer,
}

pub unsafe fn ZSTDMT_freeBufferPool(bufPool: *mut ZSTDMT_bufferPool) {
    if bufPool.is_null() {
        return; /* compatibility with free on NULL */
    }
    if !(*bufPool).buffers.is_null() {
        let mut u: core::ffi::c_uint;
        u = 0;
        while u < (*bufPool).totalBuffers {
            ZSTD_customFree(
                (*(*bufPool).buffers.add(u as usize)).start as *mut u8,
                (*bufPool).cMem,
            );
            u = u.wrapping_add(1);
        }
        ZSTD_customFree((*bufPool).buffers as *mut u8, (*bufPool).cMem);
    }
    ZSTD_pthread_mutex_destroy(&mut (*bufPool).poolMutex);
    ZSTD_customFree(bufPool as *mut u8, (*bufPool).cMem);
}

pub unsafe fn ZSTDMT_createBufferPool(
    maxNbBuffers: core::ffi::c_uint,
    cMem: ZSTD_customMem,
) -> *mut ZSTDMT_bufferPool {
    let bufPool: *mut ZSTDMT_bufferPool =
        ZSTD_customCalloc(core::mem::size_of::<ZSTDMT_bufferPool>(), cMem)
            as *mut ZSTDMT_bufferPool;
    if bufPool.is_null() {
        return core::ptr::null_mut();
    }
    if ZSTD_pthread_mutex_init(&mut (*bufPool).poolMutex, core::ptr::null_mut()) != 0 {
        ZSTD_customFree(bufPool as *mut u8, cMem);
        return core::ptr::null_mut();
    }
    (*bufPool).buffers = ZSTD_customCalloc(
        (maxNbBuffers as usize) * core::mem::size_of::<Buffer>(),
        cMem,
    ) as *mut Buffer;
    if (*bufPool).buffers.is_null() {
        ZSTDMT_freeBufferPool(bufPool);
        return core::ptr::null_mut();
    }
    (*bufPool).bufferSize = 64 * (1 << 10);
    (*bufPool).totalBuffers = maxNbBuffers;
    (*bufPool).nbBuffers = 0;
    (*bufPool).cMem = cMem;
    bufPool
}

/* only works at initialization, not during compression */
pub unsafe fn ZSTDMT_sizeof_bufferPool(bufPool: *mut ZSTDMT_bufferPool) -> usize {
    let poolSize: usize = core::mem::size_of::<ZSTDMT_bufferPool>();
    let arraySize: usize = ((*bufPool).totalBuffers as usize) * core::mem::size_of::<Buffer>();
    let mut u: core::ffi::c_uint;
    let mut totalBufferSize: usize = 0;
    ZSTD_pthread_mutex_lock(&mut (*bufPool).poolMutex);
    u = 0;
    while u < (*bufPool).totalBuffers {
        totalBufferSize += (*(*bufPool).buffers.add(u as usize)).capacity;
        u = u.wrapping_add(1);
    }
    ZSTD_pthread_mutex_unlock(&mut (*bufPool).poolMutex);

    poolSize + arraySize + totalBufferSize
}

/* ZSTDMT_setBufferSize() :
 * all future buffers provided by this buffer pool will have _at least_ this size
 * note : it's better for all buffers to have same size,
 * as they become freely interchangeable, reducing malloc/free usages and memory fragmentation */
pub unsafe fn ZSTDMT_setBufferSize(bufPool: *mut ZSTDMT_bufferPool, bSize: usize) {
    ZSTD_pthread_mutex_lock(&mut (*bufPool).poolMutex);
    (*bufPool).bufferSize = bSize;
    ZSTD_pthread_mutex_unlock(&mut (*bufPool).poolMutex);
}

pub unsafe fn ZSTDMT_expandBufferPool(
    srcBufPool: *mut ZSTDMT_bufferPool,
    maxNbBuffers: core::ffi::c_uint,
) -> *mut ZSTDMT_bufferPool {
    if srcBufPool.is_null() {
        return core::ptr::null_mut();
    }
    if (*srcBufPool).totalBuffers >= maxNbBuffers {
        /* good enough */
        return srcBufPool;
    }
    /* need a larger buffer pool */
    {
        let cMem: ZSTD_customMem = (*srcBufPool).cMem;
        let bSize: usize = (*srcBufPool).bufferSize; /* forward parameters */
        let newBufPool: *mut ZSTDMT_bufferPool;
        ZSTDMT_freeBufferPool(srcBufPool);
        newBufPool = ZSTDMT_createBufferPool(maxNbBuffers, cMem);
        if newBufPool.is_null() {
            return newBufPool;
        }
        ZSTDMT_setBufferSize(newBufPool, bSize);
        return newBufPool;
    }
}

/** ZSTDMT_getBuffer() :
 *  assumption : bufPool must be valid
 * @return : a buffer, with start pointer and size
 *  note: allocation may fail, in this case, start==NULL and size==0 */
pub unsafe fn ZSTDMT_getBuffer(bufPool: *mut ZSTDMT_bufferPool) -> Buffer {
    let bSize: usize = (*bufPool).bufferSize;
    ZSTD_pthread_mutex_lock(&mut (*bufPool).poolMutex);
    if (*bufPool).nbBuffers != 0 {
        /* try to use an existing buffer */
        (*bufPool).nbBuffers = (*bufPool).nbBuffers.wrapping_sub(1);
        let buf: Buffer = *(*bufPool).buffers.add((*bufPool).nbBuffers as usize);
        let availBufferSize: usize = buf.capacity;
        *(*bufPool).buffers.add((*bufPool).nbBuffers as usize) = g_nullBuffer;
        if ((availBufferSize >= bSize) as core::ffi::c_int
            & ((availBufferSize >> 3) <= bSize) as core::ffi::c_int)
            != 0
        {
            /* large enough, but not too much */
            ZSTD_pthread_mutex_unlock(&mut (*bufPool).poolMutex);
            return buf;
        }
        /* size conditions not respected : scratch this buffer, create new one */
        ZSTD_customFree(buf.start as *mut u8, (*bufPool).cMem);
    }
    ZSTD_pthread_mutex_unlock(&mut (*bufPool).poolMutex);
    /* create new buffer */
    {
        let mut buffer = Buffer {
            start: core::ptr::null_mut(),
            capacity: 0,
        };
        let start: *mut core::ffi::c_void =
            ZSTD_customMalloc(bSize, (*bufPool).cMem) as *mut core::ffi::c_void;
        buffer.start = start; /* note : start can be NULL if malloc fails ! */
        buffer.capacity = if start.is_null() { 0 } else { bSize };
        return buffer;
    }
}

/* `#if ZSTD_RESIZE_SEQPOOL` : ZSTDMT_resizeBuffer() is compiled out (== 0) */

/* store buffer for later re-use, up to pool capacity */
pub unsafe fn ZSTDMT_releaseBuffer(bufPool: *mut ZSTDMT_bufferPool, buf: Buffer) {
    if buf.start.is_null() {
        return; /* compatible with release on NULL */
    }
    ZSTD_pthread_mutex_lock(&mut (*bufPool).poolMutex);
    if (*bufPool).nbBuffers < (*bufPool).totalBuffers {
        *(*bufPool).buffers.add((*bufPool).nbBuffers as usize) = buf; /* stored for later use */
        (*bufPool).nbBuffers = (*bufPool).nbBuffers.wrapping_add(1);
        ZSTD_pthread_mutex_unlock(&mut (*bufPool).poolMutex);
        return;
    }
    ZSTD_pthread_mutex_unlock(&mut (*bufPool).poolMutex);
    /* Reached bufferPool capacity (note: should not happen) */
    ZSTD_customFree(buf.start as *mut u8, (*bufPool).cMem);
}

/* We need 2 output buffers per worker since each dstBuff must be flushed after it is released.
 * The 3 additional buffers are as follows:
 *   1 buffer for input loading
 *   1 buffer for "next input" when submitting current one
 *   1 buffer stuck in queue */
#[inline(always)]
pub fn BUF_POOL_MAX_NB_BUFFERS(nbWorkers: core::ffi::c_uint) -> core::ffi::c_uint {
    (2u32.wrapping_mul(nbWorkers)).wrapping_add(3)
}

/* After a worker releases its rawSeqStore, it is immediately ready for reuse.
 * So we only need one seq buffer per worker. */
#[inline(always)]
pub fn SEQ_POOL_MAX_NB_BUFFERS(nbWorkers: core::ffi::c_uint) -> core::ffi::c_uint {
    nbWorkers
}

/* =====   Seq Pool Wrapper   ====== */

pub type ZSTDMT_seqPool = ZSTDMT_bufferPool;

pub unsafe fn ZSTDMT_sizeof_seqPool(seqPool: *mut ZSTDMT_seqPool) -> usize {
    ZSTDMT_sizeof_bufferPool(seqPool)
}

pub unsafe fn bufferToSeq(buffer: Buffer) -> RawSeqStore_t {
    let mut seq: RawSeqStore_t = kNullRawSeqStore;
    seq.seq = buffer.start as *mut rawSeq;
    seq.capacity = buffer.capacity / core::mem::size_of::<rawSeq>();
    seq
}

pub unsafe fn seqToBuffer(seq: RawSeqStore_t) -> Buffer {
    let mut buffer = Buffer {
        start: core::ptr::null_mut(),
        capacity: 0,
    };
    buffer.start = seq.seq as *mut core::ffi::c_void;
    buffer.capacity = seq.capacity * core::mem::size_of::<rawSeq>();
    buffer
}

pub unsafe fn ZSTDMT_getSeq(seqPool: *mut ZSTDMT_seqPool) -> RawSeqStore_t {
    if (*seqPool).bufferSize == 0 {
        return kNullRawSeqStore;
    }
    bufferToSeq(ZSTDMT_getBuffer(seqPool))
}

/* `#if ZSTD_RESIZE_SEQPOOL` : ZSTDMT_resizeSeq() is compiled out (== 0) */

pub unsafe fn ZSTDMT_releaseSeq(seqPool: *mut ZSTDMT_seqPool, seq: RawSeqStore_t) {
    ZSTDMT_releaseBuffer(seqPool, seqToBuffer(seq));
}

pub unsafe fn ZSTDMT_setNbSeq(seqPool: *mut ZSTDMT_seqPool, nbSeq: usize) {
    ZSTDMT_setBufferSize(seqPool, nbSeq * core::mem::size_of::<rawSeq>());
}

pub unsafe fn ZSTDMT_createSeqPool(
    nbWorkers: core::ffi::c_uint,
    cMem: ZSTD_customMem,
) -> *mut ZSTDMT_seqPool {
    let seqPool: *mut ZSTDMT_seqPool =
        ZSTDMT_createBufferPool(SEQ_POOL_MAX_NB_BUFFERS(nbWorkers), cMem);
    if seqPool.is_null() {
        return core::ptr::null_mut();
    }
    ZSTDMT_setNbSeq(seqPool, 0);
    seqPool
}

pub unsafe fn ZSTDMT_freeSeqPool(seqPool: *mut ZSTDMT_seqPool) {
    ZSTDMT_freeBufferPool(seqPool);
}

pub unsafe fn ZSTDMT_expandSeqPool(
    pool: *mut ZSTDMT_seqPool,
    nbWorkers: U32,
) -> *mut ZSTDMT_seqPool {
    ZSTDMT_expandBufferPool(pool, SEQ_POOL_MAX_NB_BUFFERS(nbWorkers))
}

/* =====   CCtx Pool   ===== */
/* a single CCtx Pool can be invoked from multiple threads in parallel */

#[repr(C)]
pub struct ZSTDMT_CCtxPool {
    pub poolMutex: ZSTD_pthread_mutex_t,
    pub totalCCtx: core::ffi::c_int,
    pub availCCtx: core::ffi::c_int,
    pub cMem: ZSTD_customMem,
    pub cctxs: *mut *mut ZSTD_CCtx,
}

/* note : all CCtx borrowed from the pool must be reverted back to the pool _before_ freeing the pool */
pub unsafe fn ZSTDMT_freeCCtxPool(pool: *mut ZSTDMT_CCtxPool) {
    if pool.is_null() {
        return;
    }
    ZSTD_pthread_mutex_destroy(&mut (*pool).poolMutex);
    if !(*pool).cctxs.is_null() {
        let mut cid: core::ffi::c_int;
        cid = 0;
        while cid < (*pool).totalCCtx {
            crate::zstd_compress::ZSTD_freeCCtx(*(*pool).cctxs.offset(cid as isize)); /* free compatible with NULL */
            cid += 1;
        }
        ZSTD_customFree((*pool).cctxs as *mut u8, (*pool).cMem);
    }
    ZSTD_customFree(pool as *mut u8, (*pool).cMem);
}

/* ZSTDMT_createCCtxPool() :
 * implies nbWorkers >= 1 , checked by caller ZSTDMT_createCCtx() */
pub unsafe fn ZSTDMT_createCCtxPool(
    nbWorkers: core::ffi::c_int,
    cMem: ZSTD_customMem,
) -> *mut ZSTDMT_CCtxPool {
    let cctxPool: *mut ZSTDMT_CCtxPool =
        ZSTD_customCalloc(core::mem::size_of::<ZSTDMT_CCtxPool>(), cMem) as *mut ZSTDMT_CCtxPool;
    if cctxPool.is_null() {
        return core::ptr::null_mut();
    }
    if ZSTD_pthread_mutex_init(&mut (*cctxPool).poolMutex, core::ptr::null_mut()) != 0 {
        ZSTD_customFree(cctxPool as *mut u8, cMem);
        return core::ptr::null_mut();
    }
    (*cctxPool).totalCCtx = nbWorkers;
    (*cctxPool).cctxs = ZSTD_customCalloc(
        (nbWorkers as usize) * core::mem::size_of::<*mut ZSTD_CCtx>(),
        cMem,
    ) as *mut *mut ZSTD_CCtx;
    if (*cctxPool).cctxs.is_null() {
        ZSTDMT_freeCCtxPool(cctxPool);
        return core::ptr::null_mut();
    }
    (*cctxPool).cMem = cMem;
    *(*cctxPool).cctxs.offset(0) = crate::zstd_compress::ZSTD_createCCtx_advanced(cMem);
    if (*(*cctxPool).cctxs.offset(0)).is_null() {
        ZSTDMT_freeCCtxPool(cctxPool);
        return core::ptr::null_mut();
    }
    (*cctxPool).availCCtx = 1; /* at least one cctx for single-thread mode */
    cctxPool
}

pub unsafe fn ZSTDMT_expandCCtxPool(
    srcPool: *mut ZSTDMT_CCtxPool,
    nbWorkers: core::ffi::c_int,
) -> *mut ZSTDMT_CCtxPool {
    if srcPool.is_null() {
        return core::ptr::null_mut();
    }
    if nbWorkers <= (*srcPool).totalCCtx {
        return srcPool; /* good enough */
    }
    /* need a larger cctx pool */
    {
        let cMem: ZSTD_customMem = (*srcPool).cMem;
        ZSTDMT_freeCCtxPool(srcPool);
        return ZSTDMT_createCCtxPool(nbWorkers, cMem);
    }
}

/* only works during initialization phase, not during compression */
pub unsafe fn ZSTDMT_sizeof_CCtxPool(cctxPool: *mut ZSTDMT_CCtxPool) -> usize {
    ZSTD_pthread_mutex_lock(&mut (*cctxPool).poolMutex);
    {
        let nbWorkers: core::ffi::c_uint = (*cctxPool).totalCCtx as core::ffi::c_uint;
        let poolSize: usize = core::mem::size_of::<ZSTDMT_CCtxPool>();
        let arraySize: usize =
            ((*cctxPool).totalCCtx as usize) * core::mem::size_of::<*mut ZSTD_CCtx>();
        let mut totalCCtxSize: usize = 0;
        let mut u: core::ffi::c_uint;
        u = 0;
        while u < nbWorkers {
            totalCCtxSize +=
                crate::zstd_compress::ZSTD_sizeof_CCtx(*(*cctxPool).cctxs.add(u as usize));
            u = u.wrapping_add(1);
        }
        ZSTD_pthread_mutex_unlock(&mut (*cctxPool).poolMutex);
        return poolSize + arraySize + totalCCtxSize;
    }
}

pub unsafe fn ZSTDMT_getCCtx(cctxPool: *mut ZSTDMT_CCtxPool) -> *mut ZSTD_CCtx {
    ZSTD_pthread_mutex_lock(&mut (*cctxPool).poolMutex);
    if (*cctxPool).availCCtx != 0 {
        (*cctxPool).availCCtx -= 1;
        {
            let cctx: *mut ZSTD_CCtx = *(*cctxPool).cctxs.offset((*cctxPool).availCCtx as isize);
            ZSTD_pthread_mutex_unlock(&mut (*cctxPool).poolMutex);
            return cctx;
        }
    }
    ZSTD_pthread_mutex_unlock(&mut (*cctxPool).poolMutex);
    /* note : can be NULL, when creation fails ! */
    crate::zstd_compress::ZSTD_createCCtx_advanced((*cctxPool).cMem)
}

pub unsafe fn ZSTDMT_releaseCCtx(pool: *mut ZSTDMT_CCtxPool, cctx: *mut ZSTD_CCtx) {
    if cctx.is_null() {
        return; /* compatibility with release on NULL */
    }
    ZSTD_pthread_mutex_lock(&mut (*pool).poolMutex);
    if (*pool).availCCtx < (*pool).totalCCtx {
        *(*pool).cctxs.offset((*pool).availCCtx as isize) = cctx;
        (*pool).availCCtx += 1;
    } else {
        /* pool overflow : should not happen, since totalCCtx==nbWorkers */
        crate::zstd_compress::ZSTD_freeCCtx(cctx);
    }
    ZSTD_pthread_mutex_unlock(&mut (*pool).poolMutex);
}

/* ====   Serial State   ==== */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Range {
    pub start: *const core::ffi::c_void,
    pub size: usize,
}

#[repr(C)]
pub struct SerialState {
    /* All variables in the struct are protected by mutex. */
    pub mutex: ZSTD_pthread_mutex_t,
    pub cond: ZSTD_pthread_cond_t,
    pub params: ZSTD_CCtx_params,
    pub ldmState: ldmState_t,
    pub xxhState: XXH64_state_t,
    pub nextJobID: core::ffi::c_uint,
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
    jobSize: usize,
    dict: *const core::ffi::c_void,
    dictSize: usize,
    dictContentType: ZSTD_dictContentType_e,
) -> core::ffi::c_int {
    /* Adjust parameters */
    if params.ldmParams.enableLdm == ZSTD_ps_enable {
        crate::zstd_ldm::ZSTD_ldm_adjustParameters(&mut params.ldmParams, &params.cParams);
    } else {
        ZSTD_memset(
            &mut params.ldmParams as *mut ldmParams_t as *mut u8,
            0,
            core::mem::size_of::<ldmParams_t>(),
        );
    }
    (*serialState).nextJobID = 0;
    if params.fParams.checksumFlag != 0 {
        ZSTD_XXH64_reset(&mut (*serialState).xxhState, 0);
    }
    if params.ldmParams.enableLdm == ZSTD_ps_enable {
        let cMem: ZSTD_customMem = params.customMem;
        let hashLog: core::ffi::c_uint = params.ldmParams.hashLog;
        let hashSize: usize = ((1usize) << hashLog) * core::mem::size_of::<ldmEntry_t>();
        let bucketLog: core::ffi::c_uint = params
            .ldmParams
            .hashLog
            .wrapping_sub(params.ldmParams.bucketSizeLog);
        let prevBucketLog: core::ffi::c_uint = (*serialState)
            .params
            .ldmParams
            .hashLog
            .wrapping_sub((*serialState).params.ldmParams.bucketSizeLog);
        let numBuckets: usize = (1usize) << bucketLog;
        /* Size the seq pool tables */
        ZSTDMT_setNbSeq(
            seqPool,
            crate::zstd_ldm::ZSTD_ldm_getMaxNbSeq(params.ldmParams, jobSize),
        );
        /* Reset the window */
        ZSTD_window_init(&mut (*serialState).ldmState.window);
        /* Resize tables and output space if necessary. */
        if (*serialState).ldmState.hashTable.is_null()
            || (*serialState).params.ldmParams.hashLog < hashLog
        {
            ZSTD_customFree((*serialState).ldmState.hashTable as *mut u8, cMem);
            (*serialState).ldmState.hashTable =
                ZSTD_customMalloc(hashSize, cMem) as *mut ldmEntry_t;
        }
        if (*serialState).ldmState.bucketOffsets.is_null() || prevBucketLog < bucketLog {
            ZSTD_customFree((*serialState).ldmState.bucketOffsets, cMem);
            (*serialState).ldmState.bucketOffsets =
                ZSTD_customMalloc(numBuckets, cMem) as *mut BYTE;
        }
        if (*serialState).ldmState.hashTable.is_null()
            || (*serialState).ldmState.bucketOffsets.is_null()
        {
            return 1;
        }
        /* Zero the tables */
        ZSTD_memset(
            (*serialState).ldmState.hashTable as *mut u8,
            0,
            hashSize,
        );
        ZSTD_memset((*serialState).ldmState.bucketOffsets, 0, numBuckets);

        /* Update window state and fill hash table with dict */
        (*serialState).ldmState.loadedDictEnd = 0;
        if dictSize > 0 {
            if dictContentType == ZSTD_dct_rawContent {
                let dictEnd: *const BYTE = (dict as *const BYTE).add(dictSize);
                ZSTD_window_update(
                    &mut (*serialState).ldmState.window,
                    dict,
                    dictSize,
                    /* forceNonContiguous */ 0,
                );
                crate::zstd_ldm::ZSTD_ldm_fillHashTable(
                    &mut (*serialState).ldmState,
                    dict as *const BYTE,
                    dictEnd,
                    &params.ldmParams,
                );
                (*serialState).ldmState.loadedDictEnd = if params.forceWindow != 0 {
                    0
                } else {
                    ((dictEnd as isize) - ((*serialState).ldmState.window.base as isize)) as U32
                };
            } else {
                /* don't even load anything */
            }
        }

        /* Initialize serialState's copy of ldmWindow. */
        (*serialState).ldmWindow = (*serialState).ldmState.window;
    }

    (*serialState).params = params;
    (*serialState).params.jobSize = jobSize as U32 as usize;
    0
}

pub unsafe fn ZSTDMT_serialState_init(serialState: *mut SerialState) -> core::ffi::c_int {
    let mut initError: core::ffi::c_int = 0;
    ZSTD_memset(
        serialState as *mut u8,
        0,
        core::mem::size_of::<SerialState>(),
    );
    initError |= ZSTD_pthread_mutex_init(&mut (*serialState).mutex, core::ptr::null_mut());
    initError |= ZSTD_pthread_cond_init(&mut (*serialState).cond, core::ptr::null_mut());
    initError |= ZSTD_pthread_mutex_init(&mut (*serialState).ldmWindowMutex, core::ptr::null_mut());
    initError |= ZSTD_pthread_cond_init(&mut (*serialState).ldmWindowCond, core::ptr::null_mut());
    initError
}

pub unsafe fn ZSTDMT_serialState_free(serialState: *mut SerialState) {
    let cMem: ZSTD_customMem = (*serialState).params.customMem;
    ZSTD_pthread_mutex_destroy(&mut (*serialState).mutex);
    ZSTD_pthread_cond_destroy(&mut (*serialState).cond);
    ZSTD_pthread_mutex_destroy(&mut (*serialState).ldmWindowMutex);
    ZSTD_pthread_cond_destroy(&mut (*serialState).ldmWindowCond);
    ZSTD_customFree((*serialState).ldmState.hashTable as *mut u8, cMem);
    ZSTD_customFree((*serialState).ldmState.bucketOffsets, cMem);
}

pub unsafe fn ZSTDMT_serialState_genSequences(
    serialState: *mut SerialState,
    seqStore: *mut RawSeqStore_t,
    src: Range,
    jobID: core::ffi::c_uint,
) {
    /* Wait for our turn */
    ZSTD_PTHREAD_MUTEX_LOCK(&mut (*serialState).mutex);
    while (*serialState).nextJobID < jobID {
        ZSTD_pthread_cond_wait(&mut (*serialState).cond, &mut (*serialState).mutex);
    }
    /* A future job may error and skip our job */
    if (*serialState).nextJobID == jobID {
        /* It is now our turn, do any processing necessary */
        if (*serialState).params.ldmParams.enableLdm == ZSTD_ps_enable {
            let error: usize;
            ZSTD_window_update(
                &mut (*serialState).ldmState.window,
                src.start,
                src.size,
                /* forceNonContiguous */ 0,
            );
            error = crate::zstd_ldm::ZSTD_ldm_generateSequences(
                &mut (*serialState).ldmState,
                seqStore,
                &(*serialState).params.ldmParams,
                src.start,
                src.size,
            );
            /* We provide a large enough buffer to never fail. */
            /* Update ldmWindow to match the ldmState.window and signal the main
             * thread if it is waiting for a buffer.
             */
            ZSTD_PTHREAD_MUTEX_LOCK(&mut (*serialState).ldmWindowMutex);
            (*serialState).ldmWindow = (*serialState).ldmState.window;
            ZSTD_pthread_cond_signal(&mut (*serialState).ldmWindowCond);
            ZSTD_pthread_mutex_unlock(&mut (*serialState).ldmWindowMutex);
        }
        if (*serialState).params.fParams.checksumFlag != 0 && src.size > 0 {
            ZSTD_XXH64_update(&mut (*serialState).xxhState, src.start, src.size);
        }
    }
    /* Now it is the next jobs turn */
    (*serialState).nextJobID = (*serialState).nextJobID.wrapping_add(1);
    ZSTD_pthread_cond_broadcast(&mut (*serialState).cond);
    ZSTD_pthread_mutex_unlock(&mut (*serialState).mutex);
}

pub unsafe fn ZSTDMT_serialState_applySequences(
    serialState: *const SerialState, /* just for an assert() check */
    jobCCtx: *mut ZSTD_CCtx,
    seqStore: *const RawSeqStore_t,
) {
    if (*seqStore).size > 0 {
        crate::zstd_compress::ZSTD_referenceExternalSequences(
            jobCCtx,
            (*seqStore).seq,
            (*seqStore).size,
        );
    }
}

pub unsafe fn ZSTDMT_serialState_ensureFinished(
    serialState: *mut SerialState,
    jobID: core::ffi::c_uint,
    cSize: usize,
) {
    ZSTD_PTHREAD_MUTEX_LOCK(&mut (*serialState).mutex);
    if (*serialState).nextJobID <= jobID {
        (*serialState).nextJobID = jobID.wrapping_add(1);
        ZSTD_pthread_cond_broadcast(&mut (*serialState).cond);

        ZSTD_PTHREAD_MUTEX_LOCK(&mut (*serialState).ldmWindowMutex);
        ZSTD_window_clear(&mut (*serialState).ldmWindow);
        ZSTD_pthread_cond_signal(&mut (*serialState).ldmWindowCond);
        ZSTD_pthread_mutex_unlock(&mut (*serialState).ldmWindowMutex);
    }
    ZSTD_pthread_mutex_unlock(&mut (*serialState).mutex);
}

/* ------------------------------------------ */
/* =====          Worker thread         ===== */
/* ------------------------------------------ */

pub const kNullRange: Range = Range {
    start: core::ptr::null(),
    size: 0,
};

#[repr(C)]
pub struct ZSTDMT_jobDescription {
    /// SHARED - set0 by mtctx, then modified by worker AND read by mtctx
    pub consumed: usize,
    /// SHARED - set0 by mtctx, then modified by worker AND read by mtctx, then set0 by mtctx
    pub cSize: usize,
    /// Thread-safe - used by mtctx and worker
    pub job_mutex: ZSTD_pthread_mutex_t,
    /// Thread-safe - used by mtctx and worker
    pub job_cond: ZSTD_pthread_cond_t,
    /// Thread-safe - used by mtctx and (all) workers
    pub cctxPool: *mut ZSTDMT_CCtxPool,
    /// Thread-safe - used by mtctx and (all) workers
    pub bufPool: *mut ZSTDMT_bufferPool,
    /// Thread-safe - used by mtctx and (all) workers
    pub seqPool: *mut ZSTDMT_seqPool,
    /// Thread-safe - used by mtctx and (all) workers
    pub serial: *mut SerialState,
    /// set by worker (or mtctx), then read by worker & mtctx, then modified by mtctx => no barrier
    pub dstBuff: Buffer,
    /// set by mtctx, then read by worker & mtctx => no barrier
    pub prefix: Range,
    /// set by mtctx, then read by worker & mtctx => no barrier
    pub src: Range,
    /// set by mtctx, then read by worker => no barrier
    pub jobID: core::ffi::c_uint,
    /// set by mtctx, then read by worker => no barrier
    pub firstJob: core::ffi::c_uint,
    /// set by mtctx, then read by worker => no barrier
    pub lastJob: core::ffi::c_uint,
    /// set by mtctx, then read by worker => no barrier
    pub params: ZSTD_CCtx_params,
    /// set by mtctx, then read by worker => no barrier
    pub cdict: *const ZSTD_CDict,
    /// set by mtctx, then read by worker => no barrier
    pub fullFrameSize: core::ffi::c_ulonglong,
    /// used only by mtctx
    pub dstFlushed: usize,
    /// used only by mtctx
    pub frameChecksumNeeded: core::ffi::c_uint,
}

/* ZSTDMT_compressionJob() is a POOL_function type */
pub unsafe extern "C" fn ZSTDMT_compressionJob(jobDescription: *mut core::ffi::c_void) {
    let job: *mut ZSTDMT_jobDescription = jobDescription as *mut ZSTDMT_jobDescription;
    /* do not modify job->params ! copy it, modify the copy */
    let mut jobParams: ZSTD_CCtx_params = (*job).params;
    let cctx: *mut ZSTD_CCtx = ZSTDMT_getCCtx((*job).cctxPool);
    let mut rawSeqStore: RawSeqStore_t = ZSTDMT_getSeq((*job).seqPool);
    let mut dstBuff: Buffer = (*job).dstBuff;
    let mut lastCBlockSize: usize = 0;

    'endJob: {
        /* resources */
        if cctx.is_null() {
            /* JOB_ERROR(ERROR(memory_allocation)) */
            ZSTD_PTHREAD_MUTEX_LOCK(&mut (*job).job_mutex);
            (*job).cSize = ERROR(ZSTD_error_memory_allocation);
            ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
            break 'endJob;
        }
        if dstBuff.start.is_null() {
            /* streaming job : doesn't provide a dstBuffer */
            dstBuff = ZSTDMT_getBuffer((*job).bufPool);
            if dstBuff.start.is_null() {
                /* JOB_ERROR(ERROR(memory_allocation)) */
                ZSTD_PTHREAD_MUTEX_LOCK(&mut (*job).job_mutex);
                (*job).cSize = ERROR(ZSTD_error_memory_allocation);
                ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                break 'endJob;
            }
            /* this value can be read in ZSTDMT_flush, when it copies the whole job */
            (*job).dstBuff = dstBuff;
        }
        if jobParams.ldmParams.enableLdm == ZSTD_ps_enable && rawSeqStore.seq.is_null() {
            /* JOB_ERROR(ERROR(memory_allocation)) */
            ZSTD_PTHREAD_MUTEX_LOCK(&mut (*job).job_mutex);
            (*job).cSize = ERROR(ZSTD_error_memory_allocation);
            ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
            break 'endJob;
        }

        /* Don't compute the checksum for chunks, since we compute it externally,
         * but write it in the header.
         */
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
            &mut rawSeqStore,
            (*job).src,
            (*job).jobID,
        );

        if !(*job).cdict.is_null() {
            let initError: usize = crate::zstd_compress::ZSTD_compressBegin_advanced_internal(
                cctx,
                core::ptr::null(),
                0,
                ZSTD_dct_auto,
                ZSTD_dtlm_fast,
                (*job).cdict,
                &jobParams,
                (*job).fullFrameSize,
            );
            if crate::zstd_common::ZSTD_isError(initError) != 0 {
                /* JOB_ERROR(initError) */
                ZSTD_PTHREAD_MUTEX_LOCK(&mut (*job).job_mutex);
                (*job).cSize = initError;
                ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                break 'endJob;
            }
        } else {
            let pledgedSrcSize: U64 = if (*job).firstJob != 0 {
                (*job).fullFrameSize
            } else {
                (*job).src.size as core::ffi::c_ulonglong
            };
            {
                let forceWindowError: usize = crate::zstd_compress::ZSTD_CCtxParams_setParameter(
                    &mut jobParams,
                    ZSTD_c_forceMaxWindow,
                    ((*job).firstJob == 0) as core::ffi::c_int,
                );
                if crate::zstd_common::ZSTD_isError(forceWindowError) != 0 {
                    /* JOB_ERROR(forceWindowError) */
                    ZSTD_PTHREAD_MUTEX_LOCK(&mut (*job).job_mutex);
                    (*job).cSize = forceWindowError;
                    ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                    break 'endJob;
                }
            }
            if (*job).firstJob == 0 {
                let err: usize = crate::zstd_compress::ZSTD_CCtxParams_setParameter(
                    &mut jobParams,
                    ZSTD_c_deterministicRefPrefix,
                    0,
                );
                if crate::zstd_common::ZSTD_isError(err) != 0 {
                    /* JOB_ERROR(err) */
                    ZSTD_PTHREAD_MUTEX_LOCK(&mut (*job).job_mutex);
                    (*job).cSize = err;
                    ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                    break 'endJob;
                }
            }
            {
                let initError: usize = crate::zstd_compress::ZSTD_compressBegin_advanced_internal(
                    cctx,
                    (*job).prefix.start,
                    (*job).prefix.size,
                    ZSTD_dct_rawContent,
                    ZSTD_dtlm_fast,
                    core::ptr::null(), /*cdict*/
                    &jobParams,
                    pledgedSrcSize,
                );
                if crate::zstd_common::ZSTD_isError(initError) != 0 {
                    /* JOB_ERROR(initError) */
                    ZSTD_PTHREAD_MUTEX_LOCK(&mut (*job).job_mutex);
                    (*job).cSize = initError;
                    ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                    break 'endJob;
                }
            }
        }

        /* External Sequences can only be applied after CCtx initialization */
        ZSTDMT_serialState_applySequences((*job).serial, cctx, &rawSeqStore);

        if (*job).firstJob == 0 {
            /* flush and overwrite frame header when it's not first job */
            let hSize: usize = crate::zstd_compress::ZSTD_compressContinue_public(
                cctx,
                dstBuff.start,
                dstBuff.capacity,
                (*job).src.start,
                0,
            );
            if crate::zstd_common::ZSTD_isError(hSize) != 0 {
                /* JOB_ERROR(hSize) */
                ZSTD_PTHREAD_MUTEX_LOCK(&mut (*job).job_mutex);
                (*job).cSize = hSize;
                ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                break 'endJob;
            }
            crate::zstd_compress::ZSTD_invalidateRepCodes(cctx);
        }

        /* compress the entire job by smaller chunks, for better granularity */
        {
            let chunkSize: usize = 4 * (ZSTD_BLOCKSIZE_MAX as usize);
            let nbChunks: core::ffi::c_int =
                (((*job).src.size + (chunkSize - 1)) / chunkSize) as core::ffi::c_int;
            let mut ip: *const BYTE = (*job).src.start as *const BYTE;
            let ostart: *mut BYTE = dstBuff.start as *mut BYTE;
            let mut op: *mut BYTE = ostart;
            let oend: *mut BYTE = op.add(dstBuff.capacity);
            let mut chunkNb: core::ffi::c_int;
            chunkNb = 1;
            while chunkNb < nbChunks {
                let cSize: usize = crate::zstd_compress::ZSTD_compressContinue_public(
                    cctx,
                    op as *mut core::ffi::c_void,
                    (oend as usize).wrapping_sub(op as usize),
                    ip as *const core::ffi::c_void,
                    chunkSize,
                );
                if crate::zstd_common::ZSTD_isError(cSize) != 0 {
                    /* JOB_ERROR(cSize) */
                    ZSTD_PTHREAD_MUTEX_LOCK(&mut (*job).job_mutex);
                    (*job).cSize = cSize;
                    ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                    break 'endJob;
                }
                ip = ip.add(chunkSize);
                op = op.add(cSize);
                /* stats */
                ZSTD_PTHREAD_MUTEX_LOCK(&mut (*job).job_mutex);
                (*job).cSize += cSize;
                (*job).consumed = chunkSize.wrapping_mul(chunkNb as usize);
                /* warns some more data is ready to be flushed */
                ZSTD_pthread_cond_signal(&mut (*job).job_cond);
                ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                chunkNb += 1;
            }
            /* last block */
            if (((nbChunks > 0) as core::ffi::c_uint) | (*job).lastJob) != 0
            /*must output a "last block" flag*/
            {
                let lastBlockSize1: usize = (*job).src.size & (chunkSize - 1);
                let lastBlockSize: usize = if ((lastBlockSize1 == 0) as core::ffi::c_int
                    & ((*job).src.size >= chunkSize) as core::ffi::c_int)
                    != 0
                {
                    chunkSize
                } else {
                    lastBlockSize1
                };
                let cSize: usize = if (*job).lastJob != 0 {
                    crate::zstd_compress::ZSTD_compressEnd_public(
                        cctx,
                        op as *mut core::ffi::c_void,
                        (oend as usize).wrapping_sub(op as usize),
                        ip as *const core::ffi::c_void,
                        lastBlockSize,
                    )
                } else {
                    crate::zstd_compress::ZSTD_compressContinue_public(
                        cctx,
                        op as *mut core::ffi::c_void,
                        (oend as usize).wrapping_sub(op as usize),
                        ip as *const core::ffi::c_void,
                        lastBlockSize,
                    )
                };
                if crate::zstd_common::ZSTD_isError(cSize) != 0 {
                    /* JOB_ERROR(cSize) */
                    ZSTD_PTHREAD_MUTEX_LOCK(&mut (*job).job_mutex);
                    (*job).cSize = cSize;
                    ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                    break 'endJob;
                }
                lastCBlockSize = cSize;
            }
        }
        crate::zstd_compress::ZSTD_CCtx_trace(cctx, 0);
    }

    /* _endJob: */
    ZSTDMT_serialState_ensureFinished((*job).serial, (*job).jobID, (*job).cSize);
    /* release resources */
    ZSTDMT_releaseSeq((*job).seqPool, rawSeqStore);
    ZSTDMT_releaseCCtx((*job).cctxPool, cctx);
    /* report */
    ZSTD_PTHREAD_MUTEX_LOCK(&mut (*job).job_mutex);
    (*job).cSize += lastCBlockSize;
    /* when job->consumed == job->src.size , compression job is presumed completed */
    (*job).consumed = (*job).src.size;
    ZSTD_pthread_cond_signal(&mut (*job).job_cond);
    ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
}

/* ------------------------------------------ */
/* =====   Multi-threaded compression   ===== */
/* ------------------------------------------ */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct InBuff_t {
    /// read-only non-owned prefix buffer
    pub prefix: Range,
    pub buffer: Buffer,
    pub filled: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoundBuff_t {
    /// The round input buffer. All jobs get references
    /// to pieces of the buffer. ZSTDMT_tryGetInputRange()
    /// handles handing out job input buffers, and makes
    /// sure it doesn't overlap with any pieces still in use.
    pub buffer: *mut BYTE,
    /// The capacity of buffer.
    pub capacity: usize,
    /// The position of the current inBuff in the round
    /// buffer. Updated past the end if the inBuff once
    /// the inBuff is sent to the worker thread.
    /// pos <= capacity.
    pub pos: usize,
}

pub const kNullRoundBuff: RoundBuff_t = RoundBuff_t {
    buffer: core::ptr::null_mut(),
    capacity: 0,
    pos: 0,
};

pub const RSYNC_LENGTH: usize = 32;
/* Don't create chunks smaller than the zstd block size.
 * This stops us from regressing compression ratio too much,
 * and ensures our output fits in ZSTD_compressBound().
 *
 * If this is shrunk < ZSTD_BLOCKSIZELOG_MIN then
 * ZSTD_COMPRESSBOUND() will need to be updated.
 */
pub const RSYNC_MIN_BLOCK_LOG: u32 = ZSTD_BLOCKSIZELOG_MAX;
pub const RSYNC_MIN_BLOCK_SIZE: usize = 1usize << RSYNC_MIN_BLOCK_LOG;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RSyncState_t {
    pub hash: U64,
    pub hitMask: U64,
    pub primePower: U64,
}

/* `struct ZSTDMT_CCtx_s` */
#[repr(C)]
pub struct ZSTDMT_CCtx {
    pub factory: *mut POOL_ctx,
    pub jobs: *mut ZSTDMT_jobDescription,
    pub bufPool: *mut ZSTDMT_bufferPool,
    pub cctxPool: *mut ZSTDMT_CCtxPool,
    pub seqPool: *mut ZSTDMT_seqPool,
    pub params: ZSTD_CCtx_params,
    pub targetSectionSize: usize,
    pub targetPrefixSize: usize,
    /// 1 => one job is already prepared, but pool has shortage of workers. Don't create a new job.
    pub jobReady: core::ffi::c_int,
    pub inBuff: InBuff_t,
    pub roundBuff: RoundBuff_t,
    pub serial: SerialState,
    pub rsync: RSyncState_t,
    pub jobIDMask: core::ffi::c_uint,
    pub doneJobID: core::ffi::c_uint,
    pub nextJobID: core::ffi::c_uint,
    pub frameEnded: core::ffi::c_uint,
    pub allJobsCompleted: core::ffi::c_uint,
    pub frameContentSize: core::ffi::c_ulonglong,
    pub consumed: core::ffi::c_ulonglong,
    pub produced: core::ffi::c_ulonglong,
    pub cMem: ZSTD_customMem,
    pub cdictLocal: *mut ZSTD_CDict,
    pub cdict: *const ZSTD_CDict,
    /// C: `unsigned providedFactory: 1;`
    pub providedFactory: core::ffi::c_uint,
}

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
        ZSTD_pthread_mutex_destroy(&mut (*jobTable.add(jobNb as usize)).job_mutex);
        ZSTD_pthread_cond_destroy(&mut (*jobTable.add(jobNb as usize)).job_cond);
        jobNb = jobNb.wrapping_add(1);
    }
    ZSTD_customFree(jobTable as *mut u8, cMem);
}

/* ZSTDMT_allocJobsTable()
 * allocate and init a job table.
 * update *nbJobsPtr to next power of 2 value, as size of table */
pub unsafe fn ZSTDMT_createJobsTable(
    nbJobsPtr: *mut U32,
    cMem: ZSTD_customMem,
) -> *mut ZSTDMT_jobDescription {
    let nbJobsLog2: U32 = ZSTD_highbit32(*nbJobsPtr).wrapping_add(1);
    let nbJobs: U32 = 1u32 << nbJobsLog2;
    let mut jobNb: U32;
    let jobTable: *mut ZSTDMT_jobDescription = ZSTD_customCalloc(
        (nbJobs as usize) * core::mem::size_of::<ZSTDMT_jobDescription>(),
        cMem,
    ) as *mut ZSTDMT_jobDescription;
    let mut initError: core::ffi::c_int = 0;
    if jobTable.is_null() {
        return core::ptr::null_mut();
    }
    *nbJobsPtr = nbJobs;
    jobNb = 0;
    while jobNb < nbJobs {
        initError |= ZSTD_pthread_mutex_init(
            &mut (*jobTable.add(jobNb as usize)).job_mutex,
            core::ptr::null_mut(),
        );
        initError |= ZSTD_pthread_cond_init(
            &mut (*jobTable.add(jobNb as usize)).job_cond,
            core::ptr::null_mut(),
        );
        jobNb = jobNb.wrapping_add(1);
    }
    if initError != 0 {
        ZSTDMT_freeJobsTable(jobTable, nbJobs, cMem);
        return core::ptr::null_mut();
    }
    jobTable
}

pub unsafe fn ZSTDMT_expandJobsTable(mtctx: *mut ZSTDMT_CCtx, nbWorkers: U32) -> usize {
    let mut nbJobs: U32 = nbWorkers.wrapping_add(2);
    if nbJobs > (*mtctx).jobIDMask.wrapping_add(1) {
        /* need more job capacity */
        ZSTDMT_freeJobsTable(
            (*mtctx).jobs,
            (*mtctx).jobIDMask.wrapping_add(1),
            (*mtctx).cMem,
        );
        (*mtctx).jobIDMask = 0;
        (*mtctx).jobs = ZSTDMT_createJobsTable(&mut nbJobs, (*mtctx).cMem);
        if (*mtctx).jobs.is_null() {
            return ERROR(ZSTD_error_memory_allocation);
        }
        (*mtctx).jobIDMask = nbJobs.wrapping_sub(1);
    }
    0
}

/* ZSTDMT_CCtxParam_setNbWorkers():
 * Internal use only */
pub unsafe fn ZSTDMT_CCtxParam_setNbWorkers(
    params: *mut ZSTD_CCtx_params,
    nbWorkers: core::ffi::c_uint,
) -> usize {
    crate::zstd_compress::ZSTD_CCtxParams_setParameter(
        params,
        ZSTD_c_nbWorkers,
        nbWorkers as core::ffi::c_int,
    )
}

/* MEM_STATIC */
pub unsafe fn ZSTDMT_createCCtx_advanced_internal(
    mut nbWorkers: core::ffi::c_uint,
    cMem: ZSTD_customMem,
    pool: *mut ZSTD_threadPool,
) -> *mut ZSTDMT_CCtx {
    let mtctx: *mut ZSTDMT_CCtx;
    let mut nbJobs: U32 = nbWorkers.wrapping_add(2);
    let initError: core::ffi::c_int;

    if nbWorkers < 1 {
        return core::ptr::null_mut();
    }
    nbWorkers = MIN(nbWorkers, ZSTDMT_NBWORKERS_MAX);
    if ((cMem.customAlloc.is_some() as core::ffi::c_int)
        ^ (cMem.customFree.is_some() as core::ffi::c_int))
        != 0
    {
        /* invalid custom allocator */
        return core::ptr::null_mut();
    }

    mtctx = ZSTD_customCalloc(core::mem::size_of::<ZSTDMT_CCtx>(), cMem) as *mut ZSTDMT_CCtx;
    if mtctx.is_null() {
        return core::ptr::null_mut();
    }
    ZSTDMT_CCtxParam_setNbWorkers(&mut (*mtctx).params, nbWorkers);
    (*mtctx).cMem = cMem;
    (*mtctx).allJobsCompleted = 1;
    if !pool.is_null() {
        (*mtctx).factory = pool;
        (*mtctx).providedFactory = 1;
    } else {
        (*mtctx).factory = POOL_create_advanced(nbWorkers as usize, 0, cMem);
        (*mtctx).providedFactory = 0;
    }
    (*mtctx).jobs = ZSTDMT_createJobsTable(&mut nbJobs, cMem);
    (*mtctx).jobIDMask = nbJobs.wrapping_sub(1);
    (*mtctx).bufPool = ZSTDMT_createBufferPool(BUF_POOL_MAX_NB_BUFFERS(nbWorkers), cMem);
    (*mtctx).cctxPool = ZSTDMT_createCCtxPool(nbWorkers as core::ffi::c_int, cMem);
    (*mtctx).seqPool = ZSTDMT_createSeqPool(nbWorkers, cMem);
    initError = ZSTDMT_serialState_init(&mut (*mtctx).serial);
    (*mtctx).roundBuff = kNullRoundBuff;
    if ((*mtctx).factory.is_null() as core::ffi::c_int
        | (*mtctx).jobs.is_null() as core::ffi::c_int
        | (*mtctx).bufPool.is_null() as core::ffi::c_int
        | (*mtctx).cctxPool.is_null() as core::ffi::c_int
        | (*mtctx).seqPool.is_null() as core::ffi::c_int
        | initError)
        != 0
    {
        ZSTDMT_freeCCtx(mtctx);
        return core::ptr::null_mut();
    }
    mtctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_createCCtx_advanced(
    nbWorkers: core::ffi::c_uint,
    cMem: ZSTD_customMem,
    pool: *mut ZSTD_threadPool,
) -> *mut ZSTDMT_CCtx {
    /* `#ifdef ZSTD_MULTITHREAD` is FALSE in this build => `#else` branch */
    core::ptr::null_mut()
}

/* ZSTDMT_releaseAllJobResources() :
 * note : ensure all workers are killed first ! */
pub unsafe fn ZSTDMT_releaseAllJobResources(mtctx: *mut ZSTDMT_CCtx) {
    let mut jobID: core::ffi::c_uint;
    jobID = 0;
    while jobID <= (*mtctx).jobIDMask {
        /* Copy the mutex/cond out */
        let mutex: ZSTD_pthread_mutex_t = (*(*mtctx).jobs.add(jobID as usize)).job_mutex;
        let cond: ZSTD_pthread_cond_t = (*(*mtctx).jobs.add(jobID as usize)).job_cond;

        ZSTDMT_releaseBuffer(
            (*mtctx).bufPool,
            (*(*mtctx).jobs.add(jobID as usize)).dstBuff,
        );

        /* Clear the job description, but keep the mutex/cond */
        ZSTD_memset(
            (*mtctx).jobs.add(jobID as usize) as *mut u8,
            0,
            core::mem::size_of::<ZSTDMT_jobDescription>(),
        );
        (*(*mtctx).jobs.add(jobID as usize)).job_mutex = mutex;
        (*(*mtctx).jobs.add(jobID as usize)).job_cond = cond;
        jobID = jobID.wrapping_add(1);
    }
    (*mtctx).inBuff.buffer = g_nullBuffer;
    (*mtctx).inBuff.filled = 0;
    (*mtctx).allJobsCompleted = 1;
}

pub unsafe fn ZSTDMT_waitForAllJobsCompleted(mtctx: *mut ZSTDMT_CCtx) {
    while (*mtctx).doneJobID < (*mtctx).nextJobID {
        let jobID: core::ffi::c_uint = (*mtctx).doneJobID & (*mtctx).jobIDMask;
        ZSTD_PTHREAD_MUTEX_LOCK(&mut (*(*mtctx).jobs.add(jobID as usize)).job_mutex);
        while (*(*mtctx).jobs.add(jobID as usize)).consumed
            < (*(*mtctx).jobs.add(jobID as usize)).src.size
        {
            /* we want to block when waiting for data to flush */
            ZSTD_pthread_cond_wait(
                &mut (*(*mtctx).jobs.add(jobID as usize)).job_cond,
                &mut (*(*mtctx).jobs.add(jobID as usize)).job_mutex,
            );
        }
        ZSTD_pthread_mutex_unlock(&mut (*(*mtctx).jobs.add(jobID as usize)).job_mutex);
        (*mtctx).doneJobID = (*mtctx).doneJobID.wrapping_add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_freeCCtx(mtctx: *mut ZSTDMT_CCtx) -> usize {
    if mtctx.is_null() {
        return 0; /* compatible with free on NULL */
    }
    if (*mtctx).providedFactory == 0 {
        POOL_free((*mtctx).factory); /* stop and free worker threads */
    }
    ZSTDMT_releaseAllJobResources(mtctx); /* release job resources into pools first */
    ZSTDMT_freeJobsTable(
        (*mtctx).jobs,
        (*mtctx).jobIDMask.wrapping_add(1),
        (*mtctx).cMem,
    );
    ZSTDMT_freeBufferPool((*mtctx).bufPool);
    ZSTDMT_freeCCtxPool((*mtctx).cctxPool);
    ZSTDMT_freeSeqPool((*mtctx).seqPool);
    ZSTDMT_serialState_free(&mut (*mtctx).serial);
    crate::zstd_compress::ZSTD_freeCDict((*mtctx).cdictLocal);
    if !(*mtctx).roundBuff.buffer.is_null() {
        ZSTD_customFree((*mtctx).roundBuff.buffer, (*mtctx).cMem);
    }
    ZSTD_customFree(mtctx as *mut u8, (*mtctx).cMem);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_sizeof_CCtx(mtctx: *mut ZSTDMT_CCtx) -> usize {
    if mtctx.is_null() {
        return 0; /* supports sizeof NULL */
    }
    core::mem::size_of::<ZSTDMT_CCtx>()
        + POOL_sizeof((*mtctx).factory)
        + ZSTDMT_sizeof_bufferPool((*mtctx).bufPool)
        + ((*mtctx).jobIDMask.wrapping_add(1) as usize)
            * core::mem::size_of::<ZSTDMT_jobDescription>()
        + ZSTDMT_sizeof_CCtxPool((*mtctx).cctxPool)
        + ZSTDMT_sizeof_seqPool((*mtctx).seqPool)
        + crate::zstd_compress::ZSTD_sizeof_CDict((*mtctx).cdictLocal)
        + (*mtctx).roundBuff.capacity
}

/* ZSTDMT_resize() :
 * @return : error code if fails, 0 on success */
pub unsafe fn ZSTDMT_resize(mtctx: *mut ZSTDMT_CCtx, nbWorkers: core::ffi::c_uint) -> usize {
    if POOL_resize((*mtctx).factory, nbWorkers as usize) != 0 {
        return ERROR(ZSTD_error_memory_allocation);
    }
    {
        let err_code = ZSTDMT_expandJobsTable(mtctx, nbWorkers);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    (*mtctx).bufPool = ZSTDMT_expandBufferPool(
        (*mtctx).bufPool,
        BUF_POOL_MAX_NB_BUFFERS(nbWorkers),
    );
    if (*mtctx).bufPool.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    (*mtctx).cctxPool = ZSTDMT_expandCCtxPool((*mtctx).cctxPool, nbWorkers as core::ffi::c_int);
    if (*mtctx).cctxPool.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    (*mtctx).seqPool = ZSTDMT_expandSeqPool((*mtctx).seqPool, nbWorkers);
    if (*mtctx).seqPool.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
    ZSTDMT_CCtxParam_setNbWorkers(&mut (*mtctx).params, nbWorkers);
    0
}

/* ZSTDMT_updateCParams_whileCompressing() :
 *  Updates a selected set of compression parameters, remaining compatible with currently active frame.
 *  New parameters will be applied to next compression job. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_updateCParams_whileCompressing(
    mtctx: *mut ZSTDMT_CCtx,
    cctxParams: *const ZSTD_CCtx_params,
) {
    /* Do not modify windowLog while compressing */
    let saved_wlog: U32 = (*mtctx).params.cParams.windowLog;
    let compressionLevel: core::ffi::c_int = (*cctxParams).compressionLevel;
    (*mtctx).params.compressionLevel = compressionLevel;
    {
        let mut cParams: ZSTD_compressionParameters =
            crate::zstd_compress::ZSTD_getCParamsFromCCtxParams(
                cctxParams,
                ZSTD_CONTENTSIZE_UNKNOWN,
                0,
                ZSTD_cpm_noAttachDict,
            );
        cParams.windowLog = saved_wlog;
        (*mtctx).params.cParams = cParams;
    }
}

/* ZSTDMT_getFrameProgression():
 * tells how much data has been consumed (input) and produced (output) for current frame.
 * able to count progression inside worker threads.
 * Note : mutex will be acquired during statistics collection inside workers. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_getFrameProgression(
    mtctx: *mut ZSTDMT_CCtx,
) -> ZSTD_frameProgression {
    let mut fps = ZSTD_frameProgression {
        ingested: 0,
        consumed: 0,
        produced: 0,
        flushed: 0,
        currentJobID: 0,
        nbActiveWorkers: 0,
    };
    fps.ingested = (*mtctx).consumed + (*mtctx).inBuff.filled as core::ffi::c_ulonglong;
    fps.consumed = (*mtctx).consumed;
    fps.produced = (*mtctx).produced;
    fps.flushed = fps.produced;
    fps.currentJobID = (*mtctx).nextJobID;
    fps.nbActiveWorkers = 0;
    {
        let mut jobNb: core::ffi::c_uint;
        let lastJobNb: core::ffi::c_uint =
            (*mtctx).nextJobID.wrapping_add((*mtctx).jobReady as core::ffi::c_uint);
        jobNb = (*mtctx).doneJobID;
        while jobNb < lastJobNb {
            let wJobID: core::ffi::c_uint = jobNb & (*mtctx).jobIDMask;
            let jobPtr: *mut ZSTDMT_jobDescription = (*mtctx).jobs.add(wJobID as usize);
            ZSTD_pthread_mutex_lock(&mut (*jobPtr).job_mutex);
            {
                let cResult: usize = (*jobPtr).cSize;
                let produced: usize = if crate::zstd_common::ZSTD_isError(cResult) != 0 {
                    0
                } else {
                    cResult
                };
                let flushed: usize = if crate::zstd_common::ZSTD_isError(cResult) != 0 {
                    0
                } else {
                    (*jobPtr).dstFlushed
                };
                fps.ingested += (*jobPtr).src.size as core::ffi::c_ulonglong;
                fps.consumed += (*jobPtr).consumed as core::ffi::c_ulonglong;
                fps.produced += produced as core::ffi::c_ulonglong;
                fps.flushed += flushed as core::ffi::c_ulonglong;
                fps.nbActiveWorkers = fps.nbActiveWorkers.wrapping_add(
                    ((*jobPtr).consumed < (*jobPtr).src.size) as core::ffi::c_uint,
                );
            }
            ZSTD_pthread_mutex_unlock(
                &mut (*(*mtctx).jobs.add(wJobID as usize)).job_mutex,
            );
            jobNb = jobNb.wrapping_add(1);
        }
    }
    fps
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_toFlushNow(mtctx: *mut ZSTDMT_CCtx) -> usize {
    let toFlush: usize;
    let jobID: core::ffi::c_uint = (*mtctx).doneJobID;
    if jobID == (*mtctx).nextJobID {
        return 0; /* no active job => nothing to flush */
    }

    /* look into oldest non-fully-flushed job */
    {
        let wJobID: core::ffi::c_uint = jobID & (*mtctx).jobIDMask;
        let jobPtr: *mut ZSTDMT_jobDescription = (*mtctx).jobs.add(wJobID as usize);
        ZSTD_pthread_mutex_lock(&mut (*jobPtr).job_mutex);
        {
            let cResult: usize = (*jobPtr).cSize;
            let produced: usize = if crate::zstd_common::ZSTD_isError(cResult) != 0 {
                0
            } else {
                cResult
            };
            let flushed: usize = if crate::zstd_common::ZSTD_isError(cResult) != 0 {
                0
            } else {
                (*jobPtr).dstFlushed
            };
            toFlush = produced - flushed;
            /* if toFlush==0, nothing is available to flush.
             * However, jobID is expected to still be active:
             * if jobID was already completed and fully flushed,
             * ZSTDMT_flushProduced() should have already moved onto next job.
             * Therefore, some input has not yet been consumed. */
        }
        ZSTD_pthread_mutex_unlock(&mut (*(*mtctx).jobs.add(wJobID as usize)).job_mutex);
    }

    toFlush
}

/* ------------------------------------------ */
/* =====   Multi-threaded compression   ===== */
/* ------------------------------------------ */

pub unsafe fn ZSTDMT_computeTargetJobLog(params: *const ZSTD_CCtx_params) -> core::ffi::c_uint {
    let jobLog: core::ffi::c_uint;
    if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
        /* In Long Range Mode, the windowLog is typically oversized.
         * In which case, it's preferable to determine the jobSize
         * based on cycleLog instead. */
        jobLog = MAX(
            21u32,
            crate::zstd_compress::ZSTD_cycleLog(
                (*params).cParams.chainLog,
                (*params).cParams.strategy,
            )
            .wrapping_add(3),
        );
    } else {
        jobLog = MAX(20u32, (*params).cParams.windowLog.wrapping_add(2));
    }
    MIN(jobLog, ZSTDMT_JOBLOG_MAX as core::ffi::c_uint)
}

pub fn ZSTDMT_overlapLog_default(strat: ZSTD_strategy) -> core::ffi::c_int {
    match strat {
        ZSTD_btultra2 => return 9,
        ZSTD_btultra | ZSTD_btopt => return 8,
        ZSTD_btlazy2 | ZSTD_lazy2 => return 7,
        /* ZSTD_lazy, ZSTD_greedy, ZSTD_dfast, ZSTD_fast, default: */
        _ => {}
    }
    6
}

pub fn ZSTDMT_overlapLog(ovlog: core::ffi::c_int, strat: ZSTD_strategy) -> core::ffi::c_int {
    if ovlog == 0 {
        return ZSTDMT_overlapLog_default(strat);
    }
    ovlog
}

pub unsafe fn ZSTDMT_computeOverlapSize(params: *const ZSTD_CCtx_params) -> usize {
    let overlapRLog: core::ffi::c_int =
        9 - ZSTDMT_overlapLog((*params).overlapLog, (*params).cParams.strategy);
    let mut ovLog: core::ffi::c_int = if overlapRLog >= 8 {
        0
    } else {
        (*params)
            .cParams
            .windowLog
            .wrapping_sub(overlapRLog as core::ffi::c_uint) as core::ffi::c_int
    };
    if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
        /* In Long Range Mode, the windowLog is typically oversized.
         * In which case, it's preferable to determine the jobSize
         * based on chainLog instead.
         * Then, ovLog becomes a fraction of the jobSize, rather than windowSize */
        ovLog = MIN(
            (*params).cParams.windowLog,
            ZSTDMT_computeTargetJobLog(params).wrapping_sub(2),
        )
        .wrapping_sub(overlapRLog as core::ffi::c_uint) as core::ffi::c_int;
    }
    if ovLog == 0 {
        0
    } else {
        (1usize) << ovLog
    }
}

/* ====================================== */
/* =======      Streaming API     ======= */
/* ====================================== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_initCStream_internal(
    mtctx: *mut ZSTDMT_CCtx,
    dict: *const core::ffi::c_void,
    dictSize: usize,
    dictContentType: ZSTD_dictContentType_e,
    cdict: *const ZSTD_CDict,
    mut params: ZSTD_CCtx_params,
    pledgedSrcSize: core::ffi::c_ulonglong,
) -> usize {
    /* params supposed partially fully validated at this point */

    /* init */
    if params.nbWorkers != (*mtctx).params.nbWorkers {
        let err_code = ZSTDMT_resize(mtctx, params.nbWorkers as core::ffi::c_uint);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }

    if params.jobSize != 0 && params.jobSize < ZSTDMT_JOBSIZE_MIN {
        params.jobSize = ZSTDMT_JOBSIZE_MIN;
    }
    if params.jobSize > ZSTDMT_JOBSIZE_MAX as usize {
        params.jobSize = ZSTDMT_JOBSIZE_MAX as usize;
    }

    if (*mtctx).allJobsCompleted == 0 {
        /* previous compression not correctly finished */
        ZSTDMT_waitForAllJobsCompleted(mtctx);
        ZSTDMT_releaseAllJobResources(mtctx);
        (*mtctx).allJobsCompleted = 1;
    }

    (*mtctx).params = params;
    (*mtctx).frameContentSize = pledgedSrcSize;
    crate::zstd_compress::ZSTD_freeCDict((*mtctx).cdictLocal);
    if !dict.is_null() {
        (*mtctx).cdictLocal = crate::zstd_compress::ZSTD_createCDict_advanced(
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
        (*mtctx).cdictLocal = core::ptr::null_mut();
        (*mtctx).cdict = cdict;
    }

    (*mtctx).targetPrefixSize = ZSTDMT_computeOverlapSize(&params);
    (*mtctx).targetSectionSize = params.jobSize;
    if (*mtctx).targetSectionSize == 0 {
        (*mtctx).targetSectionSize = (1u64 << ZSTDMT_computeTargetJobLog(&params)) as usize;
    }

    if params.rsyncable != 0 {
        /* Aim for the targetsectionSize as the average job size. */
        let jobSizeKB: U32 = ((*mtctx).targetSectionSize >> 10) as U32;
        let rsyncBits: U32 = ZSTD_highbit32(jobSizeKB).wrapping_add(10);
        /* We refuse to create jobs < RSYNC_MIN_BLOCK_SIZE bytes, so make sure our
         * expected job size is at least 4x larger. */
        (*mtctx).rsync.hash = 0;
        (*mtctx).rsync.hitMask = (1u64 << rsyncBits) - 1;
        (*mtctx).rsync.primePower = ZSTD_rollingHash_primePower(RSYNC_LENGTH as U32);
    }
    if (*mtctx).targetSectionSize < (*mtctx).targetPrefixSize {
        /* job size must be >= overlap size */
        (*mtctx).targetSectionSize = (*mtctx).targetPrefixSize;
    }
    ZSTDMT_setBufferSize(
        (*mtctx).bufPool,
        crate::zstd_compress::ZSTD_compressBound((*mtctx).targetSectionSize),
    );
    {
        /* If ldm is enabled we need windowSize space. */
        let windowSize: usize = if (*mtctx).params.ldmParams.enableLdm == ZSTD_ps_enable {
            (1u32 << (*mtctx).params.cParams.windowLog) as usize
        } else {
            0
        };
        /* Two buffers of slack, plus extra space for the overlap
         * This is the minimum slack that LDM works with. One extra because
         * flush might waste up to targetSectionSize-1 bytes. Another extra
         * for the overlap (if > 0), then one to fill which doesn't overlap
         * with the LDM window.
         */
        let nbSlackBuffers: usize = 2 + ((*mtctx).targetPrefixSize > 0) as usize;
        let slackSize: usize = (*mtctx).targetSectionSize * nbSlackBuffers;
        /* Compute the total size, and always have enough slack */
        let nbWorkers: usize = MAX((*mtctx).params.nbWorkers, 1) as usize;
        let sectionsSize: usize = (*mtctx).targetSectionSize * nbWorkers;
        let capacity: usize = MAX(windowSize, sectionsSize) + slackSize;
        if (*mtctx).roundBuff.capacity < capacity {
            if !(*mtctx).roundBuff.buffer.is_null() {
                ZSTD_customFree((*mtctx).roundBuff.buffer, (*mtctx).cMem);
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
    crate::zstd_compress::ZSTD_freeCDict((*mtctx).cdictLocal);
    (*mtctx).cdictLocal = core::ptr::null_mut();
    (*mtctx).cdict = core::ptr::null();
    if !dict.is_null() {
        if dictContentType == ZSTD_dct_rawContent {
            (*mtctx).inBuff.prefix.start = dict as *const BYTE as *const core::ffi::c_void;
            (*mtctx).inBuff.prefix.size = dictSize;
        } else {
            /* note : a loadPrefix becomes an internal CDict */
            (*mtctx).cdictLocal = crate::zstd_compress::ZSTD_createCDict_advanced(
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
        &mut (*mtctx).serial,
        (*mtctx).seqPool,
        params,
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

/* ZSTDMT_writeLastEmptyBlock()
 * Write a single empty block with an end-of-frame to finish a frame.
 * Job must be created from streaming variant.
 * This function is always successful if expected conditions are fulfilled.
 */
pub unsafe fn ZSTDMT_writeLastEmptyBlock(job: *mut ZSTDMT_jobDescription) {
    (*job).dstBuff = ZSTDMT_getBuffer((*job).bufPool);
    if (*job).dstBuff.start.is_null() {
        (*job).cSize = ERROR(ZSTD_error_memory_allocation);
        return;
    }
    (*job).src = kNullRange;
    (*job).cSize = crate::zstd_compress::ZSTD_writeLastEmptyBlock(
        (*job).dstBuff.start,
        (*job).dstBuff.capacity,
    );
}

pub unsafe fn ZSTDMT_createCompressionJob(
    mtctx: *mut ZSTDMT_CCtx,
    srcSize: usize,
    endOp: ZSTD_EndDirective,
) -> usize {
    let jobID: core::ffi::c_uint = (*mtctx).nextJobID & (*mtctx).jobIDMask;
    let endFrame: core::ffi::c_int = (endOp == ZSTD_e_end) as core::ffi::c_int;

    if (*mtctx).nextJobID > (*mtctx).doneJobID.wrapping_add((*mtctx).jobIDMask) {
        return 0;
    }

    if (*mtctx).jobReady == 0 {
        let src: *const BYTE = (*mtctx).inBuff.buffer.start as *const BYTE;
        (*(*mtctx).jobs.add(jobID as usize)).src.start = src as *const core::ffi::c_void;
        (*(*mtctx).jobs.add(jobID as usize)).src.size = srcSize;
        (*(*mtctx).jobs.add(jobID as usize)).prefix = (*mtctx).inBuff.prefix;
        (*(*mtctx).jobs.add(jobID as usize)).consumed = 0;
        (*(*mtctx).jobs.add(jobID as usize)).cSize = 0;
        (*(*mtctx).jobs.add(jobID as usize)).params = (*mtctx).params;
        (*(*mtctx).jobs.add(jobID as usize)).cdict = if (*mtctx).nextJobID == 0 {
            (*mtctx).cdict
        } else {
            core::ptr::null()
        };
        (*(*mtctx).jobs.add(jobID as usize)).fullFrameSize = (*mtctx).frameContentSize;
        (*(*mtctx).jobs.add(jobID as usize)).dstBuff = g_nullBuffer;
        (*(*mtctx).jobs.add(jobID as usize)).cctxPool = (*mtctx).cctxPool;
        (*(*mtctx).jobs.add(jobID as usize)).bufPool = (*mtctx).bufPool;
        (*(*mtctx).jobs.add(jobID as usize)).seqPool = (*mtctx).seqPool;
        (*(*mtctx).jobs.add(jobID as usize)).serial = &mut (*mtctx).serial;
        (*(*mtctx).jobs.add(jobID as usize)).jobID = (*mtctx).nextJobID;
        (*(*mtctx).jobs.add(jobID as usize)).firstJob =
            ((*mtctx).nextJobID == 0) as core::ffi::c_uint;
        (*(*mtctx).jobs.add(jobID as usize)).lastJob = endFrame as core::ffi::c_uint;
        (*(*mtctx).jobs.add(jobID as usize)).frameChecksumNeeded =
            ((*mtctx).params.fParams.checksumFlag != 0
                && endFrame != 0
                && ((*mtctx).nextJobID > 0)) as core::ffi::c_uint;
        (*(*mtctx).jobs.add(jobID as usize)).dstFlushed = 0;

        /* Update the round buffer pos and clear the input buffer to be reset */
        (*mtctx).roundBuff.pos += srcSize;
        (*mtctx).inBuff.buffer = g_nullBuffer;
        (*mtctx).inBuff.filled = 0;
        /* Set the prefix for next job */
        if endFrame == 0 {
            let newPrefixSize: usize = MIN(srcSize, (*mtctx).targetPrefixSize);
            (*mtctx).inBuff.prefix.start = src
                .wrapping_add(srcSize)
                .wrapping_sub(newPrefixSize) as *const core::ffi::c_void;
            (*mtctx).inBuff.prefix.size = newPrefixSize;
        } else {
            /* endFrame==1 => no need for another input buffer */
            (*mtctx).inBuff.prefix = kNullRange;
            (*mtctx).frameEnded = endFrame as core::ffi::c_uint;
            if (*mtctx).nextJobID == 0 {
                /* single job exception : checksum is already calculated directly within worker thread */
                (*mtctx).params.fParams.checksumFlag = 0;
            }
        }

        if (srcSize == 0) && ((*mtctx).nextJobID > 0)
        /*single job must also write frame header*/
        {
            ZSTDMT_writeLastEmptyBlock((*mtctx).jobs.add(jobID as usize));
            (*mtctx).nextJobID = (*mtctx).nextJobID.wrapping_add(1);
            return 0;
        }
    }

    if POOL_tryAdd(
        (*mtctx).factory,
        Some(ZSTDMT_compressionJob),
        (*mtctx).jobs.add(jobID as usize) as *mut core::ffi::c_void,
    ) != 0
    {
        (*mtctx).nextJobID = (*mtctx).nextJobID.wrapping_add(1);
        (*mtctx).jobReady = 0;
    } else {
        (*mtctx).jobReady = 1;
    }
    0
}

/* ZSTDMT_flushProduced() :
 *  flush whatever data has been produced but not yet flushed in current job.
 *  move to next job if current one is fully flushed.
 * `output` : `pos` will be updated with amount of data flushed .
 * `blockToFlush` : if >0, the function will block and wait if there is no data available to flush .
 * @return : amount of data remaining within internal buffer, 0 if no more, 1 if unknown but > 0, or an error code */
pub unsafe fn ZSTDMT_flushProduced(
    mtctx: *mut ZSTDMT_CCtx,
    output: *mut ZSTD_outBuffer,
    blockToFlush: core::ffi::c_uint,
    end: ZSTD_EndDirective,
) -> usize {
    let wJobID: core::ffi::c_uint = (*mtctx).doneJobID & (*mtctx).jobIDMask;

    ZSTD_PTHREAD_MUTEX_LOCK(&mut (*(*mtctx).jobs.add(wJobID as usize)).job_mutex);
    if blockToFlush != 0 && ((*mtctx).doneJobID < (*mtctx).nextJobID) {
        while (*(*mtctx).jobs.add(wJobID as usize)).dstFlushed
            == (*(*mtctx).jobs.add(wJobID as usize)).cSize
        {
            /* nothing to flush */
            if (*(*mtctx).jobs.add(wJobID as usize)).consumed
                == (*(*mtctx).jobs.add(wJobID as usize)).src.size
            {
                break;
            }
            /* block when nothing to flush but some to come */
            ZSTD_pthread_cond_wait(
                &mut (*(*mtctx).jobs.add(wJobID as usize)).job_cond,
                &mut (*(*mtctx).jobs.add(wJobID as usize)).job_mutex,
            );
        }
    }

    /* try to flush something */
    {
        /* shared */
        let mut cSize: usize = (*(*mtctx).jobs.add(wJobID as usize)).cSize;
        /* shared */
        let srcConsumed: usize = (*(*mtctx).jobs.add(wJobID as usize)).consumed;
        /* read-only, could be done after mutex lock, but no-declaration-after-statement */
        let srcSize: usize = (*(*mtctx).jobs.add(wJobID as usize)).src.size;
        ZSTD_pthread_mutex_unlock(&mut (*(*mtctx).jobs.add(wJobID as usize)).job_mutex);
        if crate::zstd_common::ZSTD_isError(cSize) != 0 {
            ZSTDMT_waitForAllJobsCompleted(mtctx);
            ZSTDMT_releaseAllJobResources(mtctx);
            return cSize;
        }
        /* add frame checksum if necessary (can only happen once) */
        if (srcConsumed == srcSize) /* job completed -> worker no longer active */
            && (*(*mtctx).jobs.add(wJobID as usize)).frameChecksumNeeded != 0
        {
            let checksum: U32 = ZSTD_XXH64_digest(&(*mtctx).serial.xxhState) as U32;
            MEM_writeLE32(
                ((*(*mtctx).jobs.add(wJobID as usize)).dstBuff.start as *mut core::ffi::c_char)
                    .add((*(*mtctx).jobs.add(wJobID as usize)).cSize) as *mut u8,
                checksum,
            );
            cSize += 4;
            /* can write this shared value, as worker is no longer active */
            (*(*mtctx).jobs.add(wJobID as usize)).cSize += 4;
            (*(*mtctx).jobs.add(wJobID as usize)).frameChecksumNeeded = 0;
        }

        if cSize > 0 {
            /* compression is ongoing or completed */
            let toFlush: usize = MIN(
                cSize - (*(*mtctx).jobs.add(wJobID as usize)).dstFlushed,
                (*output).size - (*output).pos,
            );
            if toFlush > 0 {
                ZSTD_memcpy(
                    ((*output).dst as *mut core::ffi::c_char).add((*output).pos) as *mut u8,
                    ((*(*mtctx).jobs.add(wJobID as usize)).dstBuff.start
                        as *const core::ffi::c_char)
                        .add((*(*mtctx).jobs.add(wJobID as usize)).dstFlushed)
                        as *const u8,
                    toFlush,
                );
            }
            (*output).pos += toFlush;
            /* can write : this value is only used by mtctx */
            (*(*mtctx).jobs.add(wJobID as usize)).dstFlushed += toFlush;

            if (srcConsumed == srcSize)    /* job is completed */
                && ((*(*mtctx).jobs.add(wJobID as usize)).dstFlushed == cSize)
            {
                /* output buffer fully flushed => free this job position */
                ZSTDMT_releaseBuffer(
                    (*mtctx).bufPool,
                    (*(*mtctx).jobs.add(wJobID as usize)).dstBuff,
                );
                (*(*mtctx).jobs.add(wJobID as usize)).dstBuff = g_nullBuffer;
                /* ensure this job slot is considered "not started" in future check */
                (*(*mtctx).jobs.add(wJobID as usize)).cSize = 0;
                (*mtctx).consumed += srcSize as core::ffi::c_ulonglong;
                (*mtctx).produced += cSize as core::ffi::c_ulonglong;
                (*mtctx).doneJobID = (*mtctx).doneJobID.wrapping_add(1);
            }
        }

        /* return value : how many bytes left in buffer ; fake it to 1 when unknown but >0 */
        if cSize > (*(*mtctx).jobs.add(wJobID as usize)).dstFlushed {
            return cSize - (*(*mtctx).jobs.add(wJobID as usize)).dstFlushed;
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
    /* all jobs are entirely flushed => if this one is last one, frame is completed */
    (*mtctx).allJobsCompleted = (*mtctx).frameEnded;
    if end == ZSTD_e_end {
        /* for ZSTD_e_end, question becomes : is frame completed ? instead of : are internal buffers fully flushed ? */
        return ((*mtctx).frameEnded == 0) as usize;
    }
    0 /* internal buffers fully flushed */
}

/**
 * Returns the range of data used by the earliest job that is not yet complete.
 * If the data of the first job is broken up into two segments, we cover both
 * sections.
 */
pub unsafe fn ZSTDMT_getInputDataInUse(mtctx: *mut ZSTDMT_CCtx) -> Range {
    let firstJobID: core::ffi::c_uint = (*mtctx).doneJobID;
    let lastJobID: core::ffi::c_uint = (*mtctx).nextJobID;
    let mut jobID: core::ffi::c_uint;

    /* no need to check during first round */
    let roundBuffCapacity: usize = (*mtctx).roundBuff.capacity;
    let nbJobs1stRoundMin: usize = roundBuffCapacity / (*mtctx).targetSectionSize;
    if (lastJobID as usize) < nbJobs1stRoundMin {
        return kNullRange;
    }

    jobID = firstJobID;
    while jobID < lastJobID {
        let wJobID: core::ffi::c_uint = jobID & (*mtctx).jobIDMask;
        let consumed: usize;

        ZSTD_PTHREAD_MUTEX_LOCK(&mut (*(*mtctx).jobs.add(wJobID as usize)).job_mutex);
        consumed = (*(*mtctx).jobs.add(wJobID as usize)).consumed;
        ZSTD_pthread_mutex_unlock(&mut (*(*mtctx).jobs.add(wJobID as usize)).job_mutex);

        if consumed < (*(*mtctx).jobs.add(wJobID as usize)).src.size {
            let mut range: Range = (*(*mtctx).jobs.add(wJobID as usize)).prefix;
            if range.size == 0 {
                /* Empty prefix */
                range = (*(*mtctx).jobs.add(wJobID as usize)).src;
            }
            /* Job source in multiple segments not supported yet */
            return range;
        }
        jobID = jobID.wrapping_add(1);
    }
    kNullRange
}

/**
 * Returns non-zero iff buffer and range overlap.
 */
pub unsafe fn ZSTDMT_isOverlapped(buffer: Buffer, range: Range) -> core::ffi::c_int {
    let bufferStart: *const BYTE = buffer.start as *const BYTE;
    let rangeStart: *const BYTE = range.start as *const BYTE;

    if rangeStart.is_null() || bufferStart.is_null() {
        return 0;
    }

    {
        let bufferEnd: *const BYTE = bufferStart.add(buffer.capacity);
        let rangeEnd: *const BYTE = rangeStart.add(range.size);

        /* Empty ranges cannot overlap */
        if bufferStart == bufferEnd || rangeStart == rangeEnd {
            return 0;
        }

        return (bufferStart < rangeEnd && rangeStart < bufferEnd) as core::ffi::c_int;
    }
}

pub unsafe fn ZSTDMT_doesOverlapWindow(
    buffer: Buffer,
    window: ZSTD_window_t,
) -> core::ffi::c_int {
    let mut extDict = Range {
        start: core::ptr::null(),
        size: 0,
    };
    let mut prefix = Range {
        start: core::ptr::null(),
        size: 0,
    };

    extDict.start =
        window.dictBase.add(window.lowLimit as usize) as *const core::ffi::c_void;
    extDict.size = window.dictLimit.wrapping_sub(window.lowLimit) as usize;

    prefix.start = window.base.add(window.dictLimit as usize) as *const core::ffi::c_void;
    prefix.size = (window.nextSrc as usize)
        .wrapping_sub(window.base.add(window.dictLimit as usize) as usize);

    (ZSTDMT_isOverlapped(buffer, extDict) != 0 || ZSTDMT_isOverlapped(buffer, prefix) != 0)
        as core::ffi::c_int
}

pub unsafe fn ZSTDMT_waitForLdmComplete(mtctx: *mut ZSTDMT_CCtx, buffer: Buffer) {
    if (*mtctx).params.ldmParams.enableLdm == ZSTD_ps_enable {
        let mutex: *mut ZSTD_pthread_mutex_t = &mut (*mtctx).serial.ldmWindowMutex;
        ZSTD_PTHREAD_MUTEX_LOCK(mutex);
        while ZSTDMT_doesOverlapWindow(buffer, (*mtctx).serial.ldmWindow) != 0 {
            ZSTD_pthread_cond_wait(&mut (*mtctx).serial.ldmWindowCond, mutex);
        }
        ZSTD_pthread_mutex_unlock(mutex);
    }
}

/**
 * Attempts to set the inBuff to the next section to fill.
 * If any part of the new section is still in use we give up.
 * Returns non-zero if the buffer is filled.
 */
pub unsafe fn ZSTDMT_tryGetInputRange(mtctx: *mut ZSTDMT_CCtx) -> core::ffi::c_int {
    let inUse: Range = ZSTDMT_getInputDataInUse(mtctx);
    let spaceLeft: usize = (*mtctx).roundBuff.capacity - (*mtctx).roundBuff.pos;
    let spaceNeeded: usize = (*mtctx).targetSectionSize;
    let mut buffer = Buffer {
        start: core::ptr::null_mut(),
        capacity: 0,
    };

    if spaceLeft < spaceNeeded {
        /* ZSTD_invalidateRepCodes() doesn't work for extDict variants.
         * Simply copy the prefix to the beginning in that case.
         */
        let start: *mut BYTE = (*mtctx).roundBuff.buffer;
        let prefixSize: usize = (*mtctx).inBuff.prefix.size;

        buffer.start = start as *mut core::ffi::c_void;
        buffer.capacity = prefixSize;
        if ZSTDMT_isOverlapped(buffer, inUse) != 0 {
            return 0;
        }
        ZSTDMT_waitForLdmComplete(mtctx, buffer);
        ZSTD_memmove(
            start,
            (*mtctx).inBuff.prefix.start as *const u8,
            prefixSize,
        );
        (*mtctx).inBuff.prefix.start = start as *const core::ffi::c_void;
        (*mtctx).roundBuff.pos = prefixSize;
    }
    buffer.start =
        (*mtctx).roundBuff.buffer.add((*mtctx).roundBuff.pos) as *mut core::ffi::c_void;
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
#[derive(Clone, Copy)]
pub struct SyncPoint {
    /// The number of bytes to load from the input.
    pub toLoad: usize,
    /// Boolean declaring if we must flush because we found a synchronization point.
    pub flush: core::ffi::c_int,
}

/**
 * Searches through the input for a synchronization point. If one is found, we
 * will instruct the caller to flush, and return the number of bytes to load.
 * Otherwise, we will load as many bytes as possible and instruct the caller
 * to continue as normal.
 */
pub unsafe fn findSynchronizationPoint(
    mtctx: *const ZSTDMT_CCtx,
    input: ZSTD_inBuffer,
) -> SyncPoint {
    let istart: *const BYTE = (input.src as *const BYTE).wrapping_add(input.pos);
    let primePower: U64 = (*mtctx).rsync.primePower;
    let hitMask: U64 = (*mtctx).rsync.hitMask;

    let mut syncPoint = SyncPoint {
        toLoad: 0,
        flush: 0,
    };
    let mut hash: U64;
    let prev: *const BYTE;
    let mut pos: usize;

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
        /* We don't emit synchronization points if it would produce too small blocks.
         * We don't have enough input to find a synchronization point, so don't look.
         */
        return syncPoint;
    }
    if (*mtctx).inBuff.filled + syncPoint.toLoad < RSYNC_LENGTH {
        /* Not enough to compute the hash.
         * We will miss any synchronization points in this RSYNC_LENGTH byte
         * window. However, since it depends only in the internal buffers, if the
         * state is already synchronized, we will remain synchronized.
         * Additionally, the probability that we miss a synchronization point is
         * low: RSYNC_LENGTH / targetSectionSize.
         */
        return syncPoint;
    }
    /* Initialize the loop variables. */
    if (*mtctx).inBuff.filled < RSYNC_MIN_BLOCK_SIZE {
        /* We don't need to scan the first RSYNC_MIN_BLOCK_SIZE positions
         * because they can't possibly be a sync point. So we can start
         * part way through the input buffer.
         */
        pos = RSYNC_MIN_BLOCK_SIZE - (*mtctx).inBuff.filled;
        if pos >= RSYNC_LENGTH {
            prev = istart.add(pos).sub(RSYNC_LENGTH);
            hash = ZSTD_rollingHash_compute(prev as *const core::ffi::c_void, RSYNC_LENGTH);
        } else {
            prev = ((*mtctx).inBuff.buffer.start as *const BYTE)
                .add((*mtctx).inBuff.filled)
                .sub(RSYNC_LENGTH);
            hash = ZSTD_rollingHash_compute(
                prev.add(pos) as *const core::ffi::c_void,
                RSYNC_LENGTH - pos,
            );
            hash = ZSTD_rollingHash_append(hash, istart as *const core::ffi::c_void, pos);
        }
    } else {
        /* We have enough bytes buffered to initialize the hash,
         * and have processed enough bytes to find a sync point.
         * Start scanning at the beginning of the input.
         */
        pos = 0;
        prev = ((*mtctx).inBuff.buffer.start as *const BYTE)
            .add((*mtctx).inBuff.filled)
            .sub(RSYNC_LENGTH);
        hash = ZSTD_rollingHash_compute(prev as *const core::ffi::c_void, RSYNC_LENGTH);
        if (hash & hitMask) == hitMask {
            /* We're already at a sync point so don't load any more until
             * we're able to flush this sync point.
             * This likely happened because the job table was full so we
             * couldn't add our job.
             */
            syncPoint.toLoad = 0;
            syncPoint.flush = 1;
            return syncPoint;
        }
    }
    /* Starting with the hash of the previous RSYNC_LENGTH bytes, roll
     * through the input. If we hit a synchronization point, then cut the
     * job off, and tell the compressor to flush the job. Otherwise, load
     * all the bytes and continue as normal.
     * If we go too long without a synchronization point (targetSectionSize)
     * then a block will be emitted anyways, but this is okay, since if we
     * are already synchronized we will remain synchronized.
     */
    while pos < syncPoint.toLoad {
        let toRemove: BYTE = if pos < RSYNC_LENGTH {
            *prev.add(pos)
        } else {
            *istart.add(pos - RSYNC_LENGTH)
        };
        /* This assert is very expensive, and Debian compiles with asserts enabled.
         * So disable it for now. We can get similar coverage by checking it at the
         * beginning & end of the loop.
         */
        hash = ZSTD_rollingHash_rotate(hash, toRemove, *istart.add(pos), primePower);
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
pub unsafe extern "C" fn ZSTDMT_nextInputSizeHint(mtctx: *const ZSTDMT_CCtx) -> usize {
    let mut hintInSize: usize = (*mtctx).targetSectionSize - (*mtctx).inBuff.filled;
    if hintInSize == 0 {
        hintInSize = (*mtctx).targetSectionSize;
    }
    hintInSize
}

/** ZSTDMT_compressStream_generic() :
 *  internal use only - exposed to be invoked from zstd_compress.c
 *  assumption : output and input are valid (pos <= size)
 * @return : minimum amount of data remaining to flush, 0 if none */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_compressStream_generic(
    mtctx: *mut ZSTDMT_CCtx,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
    mut endOp: ZSTD_EndDirective,
) -> usize {
    let mut forwardInputProgress: core::ffi::c_uint = 0;

    if ((*mtctx).frameEnded != 0) && (endOp == ZSTD_e_continue) {
        /* current frame being ended. Only flush/end are allowed */
        return ERROR(ZSTD_error_stage_wrong);
    }

    /* fill input buffer */
    if ((*mtctx).jobReady == 0) && ((*input).size > (*input).pos) {
        /* support NULL input */
        if (*mtctx).inBuff.buffer.start.is_null() {
            /* It is only possible for this operation to fail if there are
             * still compression jobs ongoing.  (both branches of the C code
             * only contained DEBUGLOG/assert) */
            ZSTDMT_tryGetInputRange(mtctx);
        }
        if !(*mtctx).inBuff.buffer.start.is_null() {
            let syncPoint: SyncPoint = findSynchronizationPoint(mtctx, *input);
            if syncPoint.flush != 0 && endOp == ZSTD_e_continue {
                endOp = ZSTD_e_flush;
            }
            ZSTD_memcpy(
                ((*mtctx).inBuff.buffer.start as *mut core::ffi::c_char)
                    .add((*mtctx).inBuff.filled) as *mut u8,
                ((*input).src as *const core::ffi::c_char).add((*input).pos) as *const u8,
                syncPoint.toLoad,
            );
            (*input).pos += syncPoint.toLoad;
            (*mtctx).inBuff.filled += syncPoint.toLoad;
            forwardInputProgress = (syncPoint.toLoad > 0) as core::ffi::c_uint;
        }
    }
    if ((*input).pos < (*input).size) && (endOp == ZSTD_e_end) {
        /* Can't end yet because the input is not fully consumed.
         * We are in one of these cases:
         * - mtctx->inBuff is NULL & empty: we couldn't get an input buffer so don't create a new job.
         * - We filled the input buffer: flush this job but don't end the frame.
         * - We hit a synchronization point: flush this job but don't end the frame.
         */
        endOp = ZSTD_e_flush;
    }

    if ((*mtctx).jobReady != 0)
        || ((*mtctx).inBuff.filled >= (*mtctx).targetSectionSize)  /* filled enough : let's compress */
        || ((endOp != ZSTD_e_continue) && ((*mtctx).inBuff.filled > 0))  /* something to flush : let's go */
        || ((endOp == ZSTD_e_end) && ((*mtctx).frameEnded == 0))
    {
        /* must finish the frame with a zero-size block */
        let jobSize: usize = (*mtctx).inBuff.filled;
        {
            let err_code = ZSTDMT_createCompressionJob(mtctx, jobSize, endOp);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
    }

    /* check for potential compressed data ready to be flushed */
    {
        /* block if there was no forward input progress */
        let remainingToFlush: usize = ZSTDMT_flushProduced(
            mtctx,
            output,
            (forwardInputProgress == 0) as core::ffi::c_uint,
            endOp,
        );
        if (*input).pos < (*input).size {
            /* input not consumed : do not end flush yet */
            return MAX(remainingToFlush, 1);
        }
        return remainingToFlush;
    }
}
