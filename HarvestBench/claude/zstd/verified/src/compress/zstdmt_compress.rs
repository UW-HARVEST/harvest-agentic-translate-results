/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

//! Translation of compress/zstdmt_compress.c.
//!
//! Build configuration: ZSTD_MULTITHREAD is NOT defined (single-threaded build).
//! In this configuration, `common/threading.h` maps ZSTD_pthread_mutex_t and
//! ZSTD_pthread_cond_t to `int` and turns every mutex/cond operation into a
//! no-op. The whole file compiles unchanged except for
//! ZSTDMT_createCCtx_advanced(), which returns NULL (the `#else` branch of the
//! single `#ifdef ZSTD_MULTITHREAD` in the file).
//!
//! Because POOL_add/POOL_tryAdd run jobs synchronously (see common/pool.rs),
//! every "job" completes before control returns, so the cond-wait loops (which
//! are no-ops here) never spin: consumed already equals src.size by the time
//! they are checked. This matches the C behavior with no-op pthreads exactly.

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(dead_code, unused_mut, unused_assignments, unused_parens, unused_variables)]

use core::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

use crate::common::allocations::{
    memcpy, memmove, memset, zstd_custom_calloc, zstd_custom_free, zstd_custom_malloc,
    ZSTD_customMem,
};
use crate::common::bits::highbit32 as ZSTD_highbit32;
use crate::common::error::{code, err_is_error, error};
use crate::common::mem::{mem_write_le32, U32, U64};
use crate::common::pool::{
    POOL_create_advanced, POOL_free, POOL_resize, POOL_sizeof, POOL_tryAdd, POOL_ctx, POOL_function,
};
use crate::common::xxhash::{XXH64_state_t, ZSTD_XXH64_digest, ZSTD_XXH64_reset, ZSTD_XXH64_update};
use crate::common::zstd_common::ZSTD_isError;
use crate::compress::zstd_compress::{
    ZSTD_compressBound, ZSTD_createCCtx_advanced, ZSTD_freeCCtx, ZSTD_sizeof_CCtx,
};
use crate::compress::zstd_compress_internal::{
    kNullRawSeqStore, ldmEntry_t, rawSeq, ldmState_t, RawSeqStore_t, ZSTD_CCtx, ZSTD_CCtx_params,
    ZSTD_CDict, ZSTD_threadPool, ZSTD_window_t, ZSTD_cpm_noAttachDict, ZSTD_ps_disable,
    ZSTD_ps_enable, ZSTD_rollingHash_compute, ZSTD_rollingHash_primePower, ZSTD_rollingHash_rotate,
    ZSTD_window_clear, ZSTD_window_hasExtDict, ZSTD_window_init, ZSTD_window_update,
};
use crate::compress::zstd_ldm::{
    ZSTD_ldm_adjustParameters, ZSTD_ldm_fillHashTable, ZSTD_ldm_generateSequences,
    ZSTD_ldm_getMaxNbSeq,
};
use crate::zstd_h::{
    ZSTD_btlazy2, ZSTD_btopt, ZSTD_btultra, ZSTD_btultra2, ZSTD_compressionParameters,
    ZSTD_dct_rawContent, ZSTD_dictContentType_e, ZSTD_e_continue, ZSTD_e_end, ZSTD_e_flush,
    ZSTD_inBuffer, ZSTD_lazy2, ZSTD_outBuffer, ZSTD_strategy,
    ZSTD_EndDirective, ZSTD_CONTENTSIZE_UNKNOWN, ZSTD_dlm_byCopy, ZSTD_dlm_byRef,
};

/* ======   Public ABI struct returned by value   ====== */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_frameProgression {
    pub ingested: c_ulonglong,
    pub consumed: c_ulonglong,
    pub produced: c_ulonglong,
    pub flushed: c_ulonglong,
    pub currentJobID: c_uint,
    pub nbActiveWorkers: c_uint,
}

/* ======   Cross-file functions declared extern "C"   ====== */
extern "C" {
    fn ZSTD_createCDict_advanced(
        dict: *const c_void,
        dictSize: usize,
        dictLoadMethod: u32,
        dictContentType: ZSTD_dictContentType_e,
        cParams: ZSTD_compressionParameters,
        customMem: ZSTD_customMem,
    ) -> *mut ZSTD_CDict;
    fn ZSTD_freeCDict(cdict: *mut ZSTD_CDict) -> usize;
    fn ZSTD_sizeof_CDict(cdict: *const ZSTD_CDict) -> usize;
    fn ZSTD_compressBegin_advanced_internal(
        cctx: *mut ZSTD_CCtx,
        dict: *const c_void,
        dictSize: usize,
        dictContentType: ZSTD_dictContentType_e,
        dtlm: u32,
        cdict: *const ZSTD_CDict,
        params: *const ZSTD_CCtx_params,
        pledgedSrcSize: c_ulonglong,
    ) -> usize;
    fn ZSTD_CCtxParams_setParameter(
        params: *mut ZSTD_CCtx_params,
        param: c_int,
        value: c_int,
    ) -> usize;
    fn ZSTD_getCParamsFromCCtxParams(
        CCtxParams: *const ZSTD_CCtx_params,
        srcSizeHint: U64,
        dictSize: usize,
        mode: u32,
    ) -> ZSTD_compressionParameters;
    fn ZSTD_compressContinue_public(
        cctx: *mut ZSTD_CCtx,
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    fn ZSTD_compressEnd_public(
        cctx: *mut ZSTD_CCtx,
        dst: *mut c_void,
        dstCapacity: usize,
        src: *const c_void,
        srcSize: usize,
    ) -> usize;
    fn ZSTD_invalidateRepCodes(cctx: *mut ZSTD_CCtx);
    fn ZSTD_referenceExternalSequences(cctx: *mut ZSTD_CCtx, seq: *mut rawSeq, nbSeq: usize);
    fn ZSTD_CCtx_trace(cctx: *mut ZSTD_CCtx, extraCSize: usize);
    fn ZSTD_writeLastEmptyBlock(dst: *mut c_void, dstCapacity: usize) -> usize;
    fn ZSTD_checkCParams(params: ZSTD_compressionParameters) -> usize;
    fn ZSTD_cycleLog(hashLog: U32, strat: ZSTD_strategy) -> U32;
}

/* ======   Constants   ====== */
const KB: usize = 1 << 10;
const MB: usize = 1 << 20;

// From zstdmt_compress.h (64-bit target)
const ZSTDMT_NBWORKERS_MAX: c_uint = 256;
const ZSTDMT_JOBSIZE_MIN: usize = 512 * KB;
const ZSTDMT_JOBLOG_MAX: c_int = 30;
const ZSTDMT_JOBSIZE_MAX: usize = 1024 * MB;

// ZSTD_cParameter values used here (from public zstd.h)
const ZSTD_c_nbWorkers: c_int = 400;
const ZSTD_c_forceMaxWindow: c_int = 1000; /* experimentalParam3 */
const ZSTD_c_deterministicRefPrefix: c_int = 1012; /* experimentalParam15 */

const ZSTD_BLOCKSIZELOG_MAX: usize = 17;
const ZSTD_BLOCKSIZE_MAX: usize = 1 << ZSTD_BLOCKSIZELOG_MAX;
const ZSTD_WINDOWLOG_MAX: c_int = 31; /* 64-bit */
const ZSTD_blockHeaderSize: usize = 3;

#[inline]
fn sizeof_rawSeq() -> usize {
    core::mem::size_of::<rawSeq>()
}

/* ======   Threading no-ops (ZSTD_MULTITHREAD undefined)   ====== */
type ZSTD_pthread_mutex_t = c_int;
type ZSTD_pthread_cond_t = c_int;

#[inline]
unsafe fn ZSTD_pthread_mutex_init(_a: *mut ZSTD_pthread_mutex_t, _b: *const c_void) -> c_int {
    0
}
#[inline]
unsafe fn ZSTD_pthread_mutex_destroy(_a: *mut ZSTD_pthread_mutex_t) {}
#[inline]
unsafe fn ZSTD_pthread_mutex_lock(_a: *mut ZSTD_pthread_mutex_t) {}
#[inline]
unsafe fn ZSTD_pthread_mutex_unlock(_a: *mut ZSTD_pthread_mutex_t) {}
#[inline]
unsafe fn ZSTD_pthread_cond_init(_a: *mut ZSTD_pthread_cond_t, _b: *const c_void) -> c_int {
    0
}
#[inline]
unsafe fn ZSTD_pthread_cond_destroy(_a: *mut ZSTD_pthread_cond_t) {}
#[inline]
unsafe fn ZSTD_pthread_cond_wait(_a: *mut ZSTD_pthread_cond_t, _b: *mut ZSTD_pthread_mutex_t) {}
#[inline]
unsafe fn ZSTD_pthread_cond_signal(_a: *mut ZSTD_pthread_cond_t) {}
#[inline]
unsafe fn ZSTD_pthread_cond_broadcast(_a: *mut ZSTD_pthread_cond_t) {}

/* copy a ZSTD_CCtx_params by value (POD, repr(C)) */
#[inline]
unsafe fn copy_params(p: *const ZSTD_CCtx_params) -> ZSTD_CCtx_params {
    core::ptr::read(p)
}

/* local copy of ZSTD_rollingHash_append (private in zstd_compress_internal). */
const prime8bytes: U64 = 0xCF1BBCDCB7A56463;
const ZSTD_ROLL_HASH_CHAR_OFFSET: U64 = 10;
#[inline]
unsafe fn ZSTD_rollingHash_append(mut hash: U64, buf: *const c_void, size: usize) -> U64 {
    let istart = buf as *const u8;
    let mut pos = 0usize;
    while pos < size {
        hash = hash.wrapping_mul(prime8bytes);
        hash = hash.wrapping_add(*istart.add(pos) as U64 + ZSTD_ROLL_HASH_CHAR_OFFSET);
        pos += 1;
    }
    hash
}

#[inline]
fn min_usize(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}
#[inline]
fn max_usize(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
    }
}
#[inline]
fn max_u32(a: u32, b: u32) -> u32 {
    if a > b {
        a
    } else {
        b
    }
}

/* =====   Buffer Pool   ===== */

#[repr(C)]
#[derive(Clone, Copy)]
struct Buffer {
    start: *mut c_void,
    capacity: usize,
}

const g_nullBuffer: Buffer = Buffer {
    start: core::ptr::null_mut(),
    capacity: 0,
};

#[repr(C)]
struct ZSTDMT_bufferPool {
    poolMutex: ZSTD_pthread_mutex_t,
    bufferSize: usize,
    totalBuffers: c_uint,
    nbBuffers: c_uint,
    cMem: ZSTD_customMem,
    buffers: *mut Buffer,
}

unsafe fn ZSTDMT_freeBufferPool(bufPool: *mut ZSTDMT_bufferPool) {
    if bufPool.is_null() {
        return; /* compatibility with free on NULL */
    }
    if !(*bufPool).buffers.is_null() {
        let mut u: c_uint = 0;
        while u < (*bufPool).totalBuffers {
            zstd_custom_free((*(*bufPool).buffers.add(u as usize)).start, (*bufPool).cMem);
            u += 1;
        }
        zstd_custom_free((*bufPool).buffers as *mut c_void, (*bufPool).cMem);
    }
    ZSTD_pthread_mutex_destroy(&mut (*bufPool).poolMutex);
    zstd_custom_free(bufPool as *mut c_void, (*bufPool).cMem);
}

unsafe fn ZSTDMT_createBufferPool(
    maxNbBuffers: c_uint,
    cMem: ZSTD_customMem,
) -> *mut ZSTDMT_bufferPool {
    let bufPool = zstd_custom_calloc(
        core::mem::size_of::<ZSTDMT_bufferPool>(),
        cMem,
    ) as *mut ZSTDMT_bufferPool;
    if bufPool.is_null() {
        return core::ptr::null_mut();
    }
    if ZSTD_pthread_mutex_init(&mut (*bufPool).poolMutex, core::ptr::null()) != 0 {
        zstd_custom_free(bufPool as *mut c_void, cMem);
        return core::ptr::null_mut();
    }
    (*bufPool).buffers = zstd_custom_calloc(
        (maxNbBuffers as usize) * core::mem::size_of::<Buffer>(),
        cMem,
    ) as *mut Buffer;
    if (*bufPool).buffers.is_null() {
        ZSTDMT_freeBufferPool(bufPool);
        return core::ptr::null_mut();
    }
    (*bufPool).bufferSize = 64 * KB;
    (*bufPool).totalBuffers = maxNbBuffers;
    (*bufPool).nbBuffers = 0;
    (*bufPool).cMem = cMem;
    bufPool
}

/* only works at initialization, not during compression */
unsafe fn ZSTDMT_sizeof_bufferPool(bufPool: *mut ZSTDMT_bufferPool) -> usize {
    let poolSize = core::mem::size_of::<ZSTDMT_bufferPool>();
    let arraySize = (*bufPool).totalBuffers as usize * core::mem::size_of::<Buffer>();
    let mut u: c_uint;
    let mut totalBufferSize: usize = 0;
    ZSTD_pthread_mutex_lock(&mut (*bufPool).poolMutex);
    u = 0;
    while u < (*bufPool).totalBuffers {
        totalBufferSize += (*(*bufPool).buffers.add(u as usize)).capacity;
        u += 1;
    }
    ZSTD_pthread_mutex_unlock(&mut (*bufPool).poolMutex);

    poolSize + arraySize + totalBufferSize
}

/* ZSTDMT_setBufferSize() :
 * all future buffers provided by this buffer pool will have _at least_ this size */
unsafe fn ZSTDMT_setBufferSize(bufPool: *mut ZSTDMT_bufferPool, bSize: usize) {
    ZSTD_pthread_mutex_lock(&mut (*bufPool).poolMutex);
    (*bufPool).bufferSize = bSize;
    ZSTD_pthread_mutex_unlock(&mut (*bufPool).poolMutex);
}

unsafe fn ZSTDMT_expandBufferPool(
    srcBufPool: *mut ZSTDMT_bufferPool,
    maxNbBuffers: c_uint,
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
        let cMem = (*srcBufPool).cMem;
        let bSize = (*srcBufPool).bufferSize; /* forward parameters */
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

/** ZSTDMT_getBuffer() :
 *  assumption : bufPool must be valid */
unsafe fn ZSTDMT_getBuffer(bufPool: *mut ZSTDMT_bufferPool) -> Buffer {
    let bSize = (*bufPool).bufferSize;
    ZSTD_pthread_mutex_lock(&mut (*bufPool).poolMutex);
    if (*bufPool).nbBuffers != 0 {
        /* try to use an existing buffer */
        (*bufPool).nbBuffers -= 1;
        let buf = *(*bufPool).buffers.add((*bufPool).nbBuffers as usize);
        let availBufferSize = buf.capacity;
        *(*bufPool).buffers.add((*bufPool).nbBuffers as usize) = g_nullBuffer;
        if ((availBufferSize >= bSize) as usize & ((availBufferSize >> 3) <= bSize) as usize) != 0 {
            /* large enough, but not too much */
            ZSTD_pthread_mutex_unlock(&mut (*bufPool).poolMutex);
            return buf;
        }
        /* size conditions not respected : scratch this buffer, create new one */
        zstd_custom_free(buf.start, (*bufPool).cMem);
    }
    ZSTD_pthread_mutex_unlock(&mut (*bufPool).poolMutex);
    /* create new buffer */
    {
        let mut buffer: Buffer = Buffer {
            start: core::ptr::null_mut(),
            capacity: 0,
        };
        let start = zstd_custom_malloc(bSize, (*bufPool).cMem);
        buffer.start = start; /* note : start can be NULL if malloc fails ! */
        buffer.capacity = if start.is_null() { 0 } else { bSize };
        buffer
    }
}

/* store buffer for later re-use, up to pool capacity */
unsafe fn ZSTDMT_releaseBuffer(bufPool: *mut ZSTDMT_bufferPool, buf: Buffer) {
    if buf.start.is_null() {
        return; /* compatible with release on NULL */
    }
    ZSTD_pthread_mutex_lock(&mut (*bufPool).poolMutex);
    if (*bufPool).nbBuffers < (*bufPool).totalBuffers {
        *(*bufPool).buffers.add((*bufPool).nbBuffers as usize) = buf; /* stored for later use */
        (*bufPool).nbBuffers += 1;
        ZSTD_pthread_mutex_unlock(&mut (*bufPool).poolMutex);
        return;
    }
    ZSTD_pthread_mutex_unlock(&mut (*bufPool).poolMutex);
    /* Reached bufferPool capacity (note: should not happen) */
    zstd_custom_free(buf.start, (*bufPool).cMem);
}

/* We need 2 output buffers per worker since each dstBuff must be flushed after
 * it is released. The 3 additional buffers are as follows:
 *   1 buffer for input loading
 *   1 buffer for "next input" when submitting current one
 *   1 buffer stuck in queue */
#[inline]
fn BUF_POOL_MAX_NB_BUFFERS(nbWorkers: c_uint) -> c_uint {
    2 * nbWorkers + 3
}

/* After a worker releases its rawSeqStore, it is immediately ready for reuse.
 * So we only need one seq buffer per worker. */
#[inline]
fn SEQ_POOL_MAX_NB_BUFFERS(nbWorkers: c_uint) -> c_uint {
    nbWorkers
}

/* =====   Seq Pool Wrapper   ====== */

type ZSTDMT_seqPool = ZSTDMT_bufferPool;

unsafe fn ZSTDMT_sizeof_seqPool(seqPool: *mut ZSTDMT_seqPool) -> usize {
    ZSTDMT_sizeof_bufferPool(seqPool)
}

unsafe fn bufferToSeq(buffer: Buffer) -> RawSeqStore_t {
    let mut seq: RawSeqStore_t = kNullRawSeqStore;
    seq.seq = buffer.start as *mut rawSeq;
    seq.capacity = buffer.capacity / sizeof_rawSeq();
    seq
}

unsafe fn seqToBuffer(seq: RawSeqStore_t) -> Buffer {
    let mut buffer: Buffer = Buffer {
        start: core::ptr::null_mut(),
        capacity: 0,
    };
    buffer.start = seq.seq as *mut c_void;
    buffer.capacity = seq.capacity * sizeof_rawSeq();
    buffer
}

unsafe fn ZSTDMT_getSeq(seqPool: *mut ZSTDMT_seqPool) -> RawSeqStore_t {
    if (*seqPool).bufferSize == 0 {
        return kNullRawSeqStore;
    }
    bufferToSeq(ZSTDMT_getBuffer(seqPool))
}

unsafe fn ZSTDMT_releaseSeq(seqPool: *mut ZSTDMT_seqPool, seq: RawSeqStore_t) {
    ZSTDMT_releaseBuffer(seqPool, seqToBuffer(seq));
}

unsafe fn ZSTDMT_setNbSeq(seqPool: *mut ZSTDMT_seqPool, nbSeq: usize) {
    ZSTDMT_setBufferSize(seqPool, nbSeq * sizeof_rawSeq());
}

unsafe fn ZSTDMT_createSeqPool(
    nbWorkers: c_uint,
    cMem: ZSTD_customMem,
) -> *mut ZSTDMT_seqPool {
    let seqPool = ZSTDMT_createBufferPool(SEQ_POOL_MAX_NB_BUFFERS(nbWorkers), cMem);
    if seqPool.is_null() {
        return core::ptr::null_mut();
    }
    ZSTDMT_setNbSeq(seqPool, 0);
    seqPool
}

unsafe fn ZSTDMT_freeSeqPool(seqPool: *mut ZSTDMT_seqPool) {
    ZSTDMT_freeBufferPool(seqPool);
}

unsafe fn ZSTDMT_expandSeqPool(pool: *mut ZSTDMT_seqPool, nbWorkers: U32) -> *mut ZSTDMT_seqPool {
    ZSTDMT_expandBufferPool(pool, SEQ_POOL_MAX_NB_BUFFERS(nbWorkers))
}

/* =====   CCtx Pool   ===== */

#[repr(C)]
struct ZSTDMT_CCtxPool {
    poolMutex: ZSTD_pthread_mutex_t,
    totalCCtx: c_int,
    availCCtx: c_int,
    cMem: ZSTD_customMem,
    cctxs: *mut *mut ZSTD_CCtx,
}

/* note : all CCtx borrowed from the pool must be reverted back to the pool
 * _before_ freeing the pool */
unsafe fn ZSTDMT_freeCCtxPool(pool: *mut ZSTDMT_CCtxPool) {
    if pool.is_null() {
        return;
    }
    ZSTD_pthread_mutex_destroy(&mut (*pool).poolMutex);
    if !(*pool).cctxs.is_null() {
        let mut cid: c_int = 0;
        while cid < (*pool).totalCCtx {
            ZSTD_freeCCtx(*(*pool).cctxs.add(cid as usize)); /* free compatible with NULL */
            cid += 1;
        }
        zstd_custom_free((*pool).cctxs as *mut c_void, (*pool).cMem);
    }
    zstd_custom_free(pool as *mut c_void, (*pool).cMem);
}

/* ZSTDMT_createCCtxPool() :
 * implies nbWorkers >= 1 , checked by caller ZSTDMT_createCCtx() */
unsafe fn ZSTDMT_createCCtxPool(nbWorkers: c_int, cMem: ZSTD_customMem) -> *mut ZSTDMT_CCtxPool {
    let cctxPool =
        zstd_custom_calloc(core::mem::size_of::<ZSTDMT_CCtxPool>(), cMem) as *mut ZSTDMT_CCtxPool;
    debug_assert!(nbWorkers > 0);
    if cctxPool.is_null() {
        return core::ptr::null_mut();
    }
    if ZSTD_pthread_mutex_init(&mut (*cctxPool).poolMutex, core::ptr::null()) != 0 {
        zstd_custom_free(cctxPool as *mut c_void, cMem);
        return core::ptr::null_mut();
    }
    (*cctxPool).totalCCtx = nbWorkers;
    (*cctxPool).cctxs = zstd_custom_calloc(
        nbWorkers as usize * core::mem::size_of::<*mut ZSTD_CCtx>(),
        cMem,
    ) as *mut *mut ZSTD_CCtx;
    if (*cctxPool).cctxs.is_null() {
        ZSTDMT_freeCCtxPool(cctxPool);
        return core::ptr::null_mut();
    }
    (*cctxPool).cMem = cMem;
    *(*cctxPool).cctxs.add(0) = ZSTD_createCCtx_advanced(cMem);
    if (*(*cctxPool).cctxs.add(0)).is_null() {
        ZSTDMT_freeCCtxPool(cctxPool);
        return core::ptr::null_mut();
    }
    (*cctxPool).availCCtx = 1; /* at least one cctx for single-thread mode */
    cctxPool
}

unsafe fn ZSTDMT_expandCCtxPool(
    srcPool: *mut ZSTDMT_CCtxPool,
    nbWorkers: c_int,
) -> *mut ZSTDMT_CCtxPool {
    if srcPool.is_null() {
        return core::ptr::null_mut();
    }
    if nbWorkers <= (*srcPool).totalCCtx {
        return srcPool; /* good enough */
    }
    /* need a larger cctx pool */
    {
        let cMem = (*srcPool).cMem;
        ZSTDMT_freeCCtxPool(srcPool);
        ZSTDMT_createCCtxPool(nbWorkers, cMem)
    }
}

/* only works during initialization phase, not during compression */
unsafe fn ZSTDMT_sizeof_CCtxPool(cctxPool: *mut ZSTDMT_CCtxPool) -> usize {
    ZSTD_pthread_mutex_lock(&mut (*cctxPool).poolMutex);
    {
        let nbWorkers = (*cctxPool).totalCCtx as c_uint;
        let poolSize = core::mem::size_of::<ZSTDMT_CCtxPool>();
        let arraySize = (*cctxPool).totalCCtx as usize * core::mem::size_of::<*mut ZSTD_CCtx>();
        let mut totalCCtxSize: usize = 0;
        let mut u: c_uint = 0;
        while u < nbWorkers {
            totalCCtxSize += ZSTD_sizeof_CCtx(*(*cctxPool).cctxs.add(u as usize));
            u += 1;
        }
        ZSTD_pthread_mutex_unlock(&mut (*cctxPool).poolMutex);
        debug_assert!(nbWorkers > 0);
        poolSize + arraySize + totalCCtxSize
    }
}

unsafe fn ZSTDMT_getCCtx(cctxPool: *mut ZSTDMT_CCtxPool) -> *mut ZSTD_CCtx {
    ZSTD_pthread_mutex_lock(&mut (*cctxPool).poolMutex);
    if (*cctxPool).availCCtx != 0 {
        (*cctxPool).availCCtx -= 1;
        {
            let cctx = *(*cctxPool).cctxs.add((*cctxPool).availCCtx as usize);
            ZSTD_pthread_mutex_unlock(&mut (*cctxPool).poolMutex);
            return cctx;
        }
    }
    ZSTD_pthread_mutex_unlock(&mut (*cctxPool).poolMutex);
    ZSTD_createCCtx_advanced((*cctxPool).cMem) /* note : can be NULL, when creation fails ! */
}

unsafe fn ZSTDMT_releaseCCtx(pool: *mut ZSTDMT_CCtxPool, cctx: *mut ZSTD_CCtx) {
    if cctx.is_null() {
        return; /* compatibility with release on NULL */
    }
    ZSTD_pthread_mutex_lock(&mut (*pool).poolMutex);
    if (*pool).availCCtx < (*pool).totalCCtx {
        *(*pool).cctxs.add((*pool).availCCtx as usize) = cctx;
        (*pool).availCCtx += 1;
    } else {
        /* pool overflow : should not happen, since totalCCtx==nbWorkers */
        ZSTD_freeCCtx(cctx);
    }
    ZSTD_pthread_mutex_unlock(&mut (*pool).poolMutex);
}

/* ====   Serial State   ==== */

#[repr(C)]
#[derive(Clone, Copy)]
struct Range {
    start: *const c_void,
    size: usize,
}

#[repr(C)]
struct SerialState {
    /* All variables in the struct are protected by mutex. */
    mutex: ZSTD_pthread_mutex_t,
    cond: ZSTD_pthread_cond_t,
    params: ZSTD_CCtx_params,
    ldmState: ldmState_t,
    xxhState: XXH64_state_t,
    nextJobID: c_uint,
    /* Protects ldmWindow. */
    ldmWindowMutex: ZSTD_pthread_mutex_t,
    ldmWindowCond: ZSTD_pthread_cond_t, /* Signaled when ldmWindow is updated */
    ldmWindow: ZSTD_window_t,           /* A thread-safe copy of ldmState.window */
}

unsafe fn ZSTDMT_serialState_reset(
    serialState: *mut SerialState,
    seqPool: *mut ZSTDMT_seqPool,
    mut params: ZSTD_CCtx_params,
    jobSize: usize,
    dict: *const c_void,
    dictSize: usize,
    dictContentType: ZSTD_dictContentType_e,
) -> c_int {
    /* Adjust parameters */
    if params.ldmParams.enableLdm == ZSTD_ps_enable {
        ZSTD_ldm_adjustParameters(&mut params.ldmParams, &params.cParams);
        debug_assert!(params.ldmParams.hashLog >= params.ldmParams.bucketSizeLog);
        debug_assert!(params.ldmParams.hashRateLog < 32);
    } else {
        memset(
            &mut params.ldmParams as *mut _ as *mut c_void,
            0,
            core::mem::size_of_val(&params.ldmParams),
        );
    }
    (*serialState).nextJobID = 0;
    if params.fParams.checksumFlag != 0 {
        ZSTD_XXH64_reset(&mut (*serialState).xxhState, 0);
    }
    if params.ldmParams.enableLdm == ZSTD_ps_enable {
        let cMem = params.customMem;
        let hashLog = params.ldmParams.hashLog;
        let hashSize = ((1usize) << hashLog) * core::mem::size_of::<ldmEntry_t>();
        let bucketLog = params.ldmParams.hashLog - params.ldmParams.bucketSizeLog;
        let prevBucketLog = (*serialState).params.ldmParams.hashLog
            - (*serialState).params.ldmParams.bucketSizeLog;
        let numBuckets = (1usize) << bucketLog;
        /* Size the seq pool tables */
        ZSTDMT_setNbSeq(seqPool, ZSTD_ldm_getMaxNbSeq(params.ldmParams, jobSize));
        /* Reset the window */
        ZSTD_window_init(&mut (*serialState).ldmState.window);
        /* Resize tables and output space if necessary. */
        if (*serialState).ldmState.hashTable.is_null()
            || (*serialState).params.ldmParams.hashLog < hashLog
        {
            zstd_custom_free((*serialState).ldmState.hashTable as *mut c_void, cMem);
            (*serialState).ldmState.hashTable =
                zstd_custom_malloc(hashSize, cMem) as *mut ldmEntry_t;
        }
        if (*serialState).ldmState.bucketOffsets.is_null() || prevBucketLog < bucketLog {
            zstd_custom_free((*serialState).ldmState.bucketOffsets as *mut c_void, cMem);
            (*serialState).ldmState.bucketOffsets =
                zstd_custom_malloc(numBuckets, cMem) as *mut u8;
        }
        if (*serialState).ldmState.hashTable.is_null()
            || (*serialState).ldmState.bucketOffsets.is_null()
        {
            return 1;
        }
        /* Zero the tables */
        memset((*serialState).ldmState.hashTable as *mut c_void, 0, hashSize);
        memset((*serialState).ldmState.bucketOffsets as *mut c_void, 0, numBuckets);

        /* Update window state and fill hash table with dict */
        (*serialState).ldmState.loadedDictEnd = 0;
        if dictSize > 0 {
            if dictContentType == ZSTD_dct_rawContent {
                let dictEnd = (dict as *const u8).add(dictSize);
                ZSTD_window_update(
                    &mut (*serialState).ldmState.window,
                    dict,
                    dictSize,
                    /* forceNonContiguous */ 0,
                );
                ZSTD_ldm_fillHashTable(
                    &mut (*serialState).ldmState,
                    dict as *const u8,
                    dictEnd,
                    &params.ldmParams,
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

    (*serialState).params = params;
    (*serialState).params.jobSize = jobSize as U32 as usize;
    0
}

unsafe fn ZSTDMT_serialState_init(serialState: *mut SerialState) -> c_int {
    let mut initError: c_int = 0;
    memset(
        serialState as *mut c_void,
        0,
        core::mem::size_of::<SerialState>(),
    );
    initError |= ZSTD_pthread_mutex_init(&mut (*serialState).mutex, core::ptr::null());
    initError |= ZSTD_pthread_cond_init(&mut (*serialState).cond, core::ptr::null());
    initError |= ZSTD_pthread_mutex_init(&mut (*serialState).ldmWindowMutex, core::ptr::null());
    initError |= ZSTD_pthread_cond_init(&mut (*serialState).ldmWindowCond, core::ptr::null());
    initError
}

unsafe fn ZSTDMT_serialState_free(serialState: *mut SerialState) {
    let cMem = (*serialState).params.customMem;
    ZSTD_pthread_mutex_destroy(&mut (*serialState).mutex);
    ZSTD_pthread_cond_destroy(&mut (*serialState).cond);
    ZSTD_pthread_mutex_destroy(&mut (*serialState).ldmWindowMutex);
    ZSTD_pthread_cond_destroy(&mut (*serialState).ldmWindowCond);
    zstd_custom_free((*serialState).ldmState.hashTable as *mut c_void, cMem);
    zstd_custom_free((*serialState).ldmState.bucketOffsets as *mut c_void, cMem);
}

unsafe fn ZSTDMT_serialState_genSequences(
    serialState: *mut SerialState,
    seqStore: *mut RawSeqStore_t,
    src: Range,
    jobID: c_uint,
) {
    /* Wait for our turn */
    ZSTD_pthread_mutex_lock(&mut (*serialState).mutex);
    while (*serialState).nextJobID < jobID {
        ZSTD_pthread_cond_wait(&mut (*serialState).cond, &mut (*serialState).mutex);
    }
    /* A future job may error and skip our job */
    if (*serialState).nextJobID == jobID {
        /* It is now our turn, do any processing necessary */
        if (*serialState).params.ldmParams.enableLdm == ZSTD_ps_enable {
            let error_code: usize;
            debug_assert!(
                !(*seqStore).seq.is_null()
                    && (*seqStore).pos == 0
                    && (*seqStore).size == 0
                    && (*seqStore).capacity > 0
            );
            debug_assert!(src.size <= (*serialState).params.jobSize as usize);
            ZSTD_window_update(
                &mut (*serialState).ldmState.window,
                src.start,
                src.size,
                /* forceNonContiguous */ 0,
            );
            error_code = ZSTD_ldm_generateSequences(
                &mut (*serialState).ldmState,
                seqStore,
                &(*serialState).params.ldmParams,
                src.start,
                src.size,
            );
            /* We provide a large enough buffer to never fail. */
            debug_assert!(ZSTD_isError(error_code) == 0);
            let _ = error_code;
            /* Update ldmWindow to match the ldmState.window and signal the main
             * thread if it is waiting for a buffer. */
            ZSTD_pthread_mutex_lock(&mut (*serialState).ldmWindowMutex);
            (*serialState).ldmWindow = (*serialState).ldmState.window;
            ZSTD_pthread_cond_signal(&mut (*serialState).ldmWindowCond);
            ZSTD_pthread_mutex_unlock(&mut (*serialState).ldmWindowMutex);
        }
        if (*serialState).params.fParams.checksumFlag != 0 && src.size > 0 {
            ZSTD_XXH64_update(&mut (*serialState).xxhState, src.start, src.size);
        }
    }
    /* Now it is the next jobs turn */
    (*serialState).nextJobID += 1;
    ZSTD_pthread_cond_broadcast(&mut (*serialState).cond);
    ZSTD_pthread_mutex_unlock(&mut (*serialState).mutex);
}

unsafe fn ZSTDMT_serialState_applySequences(
    serialState: *const SerialState, /* just for an assert() check */
    jobCCtx: *mut ZSTD_CCtx,
    seqStore: *const RawSeqStore_t,
) {
    if (*seqStore).size > 0 {
        debug_assert!((*serialState).params.ldmParams.enableLdm == ZSTD_ps_enable);
        let _ = serialState;
        debug_assert!(!jobCCtx.is_null());
        ZSTD_referenceExternalSequences(jobCCtx, (*seqStore).seq, (*seqStore).size);
    }
}

unsafe fn ZSTDMT_serialState_ensureFinished(
    serialState: *mut SerialState,
    jobID: c_uint,
    cSize: usize,
) {
    ZSTD_pthread_mutex_lock(&mut (*serialState).mutex);
    if (*serialState).nextJobID <= jobID {
        debug_assert!(ZSTD_isError(cSize) != 0);
        let _ = cSize;
        (*serialState).nextJobID = jobID + 1;
        ZSTD_pthread_cond_broadcast(&mut (*serialState).cond);

        ZSTD_pthread_mutex_lock(&mut (*serialState).ldmWindowMutex);
        ZSTD_window_clear(&mut (*serialState).ldmWindow);
        ZSTD_pthread_cond_signal(&mut (*serialState).ldmWindowCond);
        ZSTD_pthread_mutex_unlock(&mut (*serialState).ldmWindowMutex);
    }
    ZSTD_pthread_mutex_unlock(&mut (*serialState).mutex);
}

/* ------------------------------------------ */
/* =====          Worker thread         ===== */
/* ------------------------------------------ */

const kNullRange: Range = Range {
    start: core::ptr::null(),
    size: 0,
};

#[repr(C)]
struct ZSTDMT_jobDescription {
    consumed: usize, /* SHARED */
    cSize: usize,    /* SHARED */
    job_mutex: ZSTD_pthread_mutex_t,
    job_cond: ZSTD_pthread_cond_t,
    cctxPool: *mut ZSTDMT_CCtxPool,
    bufPool: *mut ZSTDMT_bufferPool,
    seqPool: *mut ZSTDMT_seqPool,
    serial: *mut SerialState,
    dstBuff: Buffer,
    prefix: Range,
    src: Range,
    jobID: c_uint,
    firstJob: c_uint,
    lastJob: c_uint,
    params: ZSTD_CCtx_params,
    cdict: *const ZSTD_CDict,
    fullFrameSize: c_ulonglong,
    dstFlushed: usize,          /* used only by mtctx */
    frameChecksumNeeded: c_uint, /* used only by mtctx */
}

/* ZSTDMT_compressionJob() is a POOL_function type */
extern "C" fn ZSTDMT_compressionJob(jobDescription: *mut c_void) {
    unsafe {
        let job = jobDescription as *mut ZSTDMT_jobDescription;
        let mut jobParams: ZSTD_CCtx_params = copy_params(&(*job).params); /* copy it, modify the copy */
        let cctx = ZSTDMT_getCCtx((*job).cctxPool);
        let mut rawSeqStore = ZSTDMT_getSeq((*job).seqPool);
        let mut dstBuff = (*job).dstBuff;
        let mut lastCBlockSize: usize = 0;

        /* resources */
        'endJob: loop {
            if cctx.is_null() {
                ZSTD_pthread_mutex_lock(&mut (*job).job_mutex);
                (*job).cSize = error(code::MEMORY_ALLOCATION);
                ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                break 'endJob;
            }
            if dstBuff.start.is_null() {
                /* streaming job : doesn't provide a dstBuffer */
                dstBuff = ZSTDMT_getBuffer((*job).bufPool);
                if dstBuff.start.is_null() {
                    ZSTD_pthread_mutex_lock(&mut (*job).job_mutex);
                    (*job).cSize = error(code::MEMORY_ALLOCATION);
                    ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                    break 'endJob;
                }
                (*job).dstBuff = dstBuff; /* read in ZSTDMT_flush */
            }
            if jobParams.ldmParams.enableLdm == ZSTD_ps_enable && rawSeqStore.seq.is_null() {
                ZSTD_pthread_mutex_lock(&mut (*job).job_mutex);
                (*job).cSize = error(code::MEMORY_ALLOCATION);
                ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                break 'endJob;
            }

            /* Don't compute the checksum for chunks, but write it in the header. */
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
                let initError = ZSTD_compressBegin_advanced_internal(
                    cctx,
                    core::ptr::null(),
                    0,
                    crate::zstd_h::ZSTD_dct_auto,
                    crate::compress::zstd_compress_internal::ZSTD_dtlm_fast,
                    (*job).cdict,
                    &jobParams,
                    (*job).fullFrameSize,
                );
                debug_assert!((*job).firstJob != 0); /* only allowed for first job */
                if ZSTD_isError(initError) != 0 {
                    ZSTD_pthread_mutex_lock(&mut (*job).job_mutex);
                    (*job).cSize = initError;
                    ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                    break 'endJob;
                }
            } else {
                let pledgedSrcSize: U64 = if (*job).firstJob != 0 {
                    (*job).fullFrameSize
                } else {
                    (*job).src.size as U64
                };
                {
                    let forceWindowError = ZSTD_CCtxParams_setParameter(
                        &mut jobParams,
                        ZSTD_c_forceMaxWindow,
                        ((*job).firstJob == 0) as c_int,
                    );
                    if ZSTD_isError(forceWindowError) != 0 {
                        ZSTD_pthread_mutex_lock(&mut (*job).job_mutex);
                        (*job).cSize = forceWindowError;
                        ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                        break 'endJob;
                    }
                }
                if (*job).firstJob == 0 {
                    let err = ZSTD_CCtxParams_setParameter(
                        &mut jobParams,
                        ZSTD_c_deterministicRefPrefix,
                        0,
                    );
                    if ZSTD_isError(err) != 0 {
                        ZSTD_pthread_mutex_lock(&mut (*job).job_mutex);
                        (*job).cSize = err;
                        ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                        break 'endJob;
                    }
                }
                {
                    let initError = ZSTD_compressBegin_advanced_internal(
                        cctx,
                        (*job).prefix.start,
                        (*job).prefix.size,
                        ZSTD_dct_rawContent,
                        crate::compress::zstd_compress_internal::ZSTD_dtlm_fast,
                        core::ptr::null(), /*cdict*/
                        &jobParams,
                        pledgedSrcSize,
                    );
                    if ZSTD_isError(initError) != 0 {
                        ZSTD_pthread_mutex_lock(&mut (*job).job_mutex);
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
                let hSize = ZSTD_compressContinue_public(
                    cctx,
                    dstBuff.start,
                    dstBuff.capacity,
                    (*job).src.start,
                    0,
                );
                if ZSTD_isError(hSize) != 0 {
                    ZSTD_pthread_mutex_lock(&mut (*job).job_mutex);
                    (*job).cSize = hSize;
                    ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                    break 'endJob;
                }
                ZSTD_invalidateRepCodes(cctx);
            }

            /* compress the entire job by smaller chunks, for better granularity */
            {
                let chunkSize: usize = 4 * ZSTD_BLOCKSIZE_MAX;
                let nbChunks: c_int =
                    (((*job).src.size + (chunkSize - 1)) / chunkSize) as c_int;
                let mut ip = (*job).src.start as *const u8;
                let ostart = dstBuff.start as *mut u8;
                let mut op = ostart;
                let oend = op.add(dstBuff.capacity);
                let mut chunkNb: c_int;
                debug_assert!((*job).cSize == 0);
                chunkNb = 1;
                let mut errored = false;
                while chunkNb < nbChunks {
                    let cSize = ZSTD_compressContinue_public(
                        cctx,
                        op as *mut c_void,
                        oend.offset_from(op) as usize,
                        ip as *const c_void,
                        chunkSize,
                    );
                    if ZSTD_isError(cSize) != 0 {
                        ZSTD_pthread_mutex_lock(&mut (*job).job_mutex);
                        (*job).cSize = cSize;
                        ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                        errored = true;
                        break;
                    }
                    ip = ip.add(chunkSize);
                    op = op.add(cSize);
                    debug_assert!(op < oend);
                    /* stats */
                    ZSTD_pthread_mutex_lock(&mut (*job).job_mutex);
                    (*job).cSize += cSize;
                    (*job).consumed = chunkSize * chunkNb as usize;
                    ZSTD_pthread_cond_signal(&mut (*job).job_cond);
                    ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                    chunkNb += 1;
                }
                if errored {
                    break 'endJob;
                }
                /* last block */
                debug_assert!(chunkSize > 0);
                debug_assert!((chunkSize & (chunkSize - 1)) == 0);
                if ((nbChunks > 0) as c_uint | (*job).lastJob) != 0 {
                    let lastBlockSize1 = (*job).src.size & (chunkSize - 1);
                    let lastBlockSize = if ((lastBlockSize1 == 0) as usize
                        & ((*job).src.size >= chunkSize) as usize)
                        != 0
                    {
                        chunkSize
                    } else {
                        lastBlockSize1
                    };
                    let cSize = if (*job).lastJob != 0 {
                        ZSTD_compressEnd_public(
                            cctx,
                            op as *mut c_void,
                            oend.offset_from(op) as usize,
                            ip as *const c_void,
                            lastBlockSize,
                        )
                    } else {
                        ZSTD_compressContinue_public(
                            cctx,
                            op as *mut c_void,
                            oend.offset_from(op) as usize,
                            ip as *const c_void,
                            lastBlockSize,
                        )
                    };
                    if ZSTD_isError(cSize) != 0 {
                        ZSTD_pthread_mutex_lock(&mut (*job).job_mutex);
                        (*job).cSize = cSize;
                        ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
                        break 'endJob;
                    }
                    lastCBlockSize = cSize;
                }
            }
            if (*job).firstJob == 0 {
                /* Double check that we don't have an ext-dict. */
                debug_assert!(
                    ZSTD_window_hasExtDict(
                        (*cctx).blockState.matchState.window
                    ) == 0
                );
            }
            ZSTD_CCtx_trace(cctx, 0);

            break 'endJob;
        }

        /* _endJob: */
        ZSTDMT_serialState_ensureFinished((*job).serial, (*job).jobID, (*job).cSize);
        /* release resources */
        ZSTDMT_releaseSeq((*job).seqPool, rawSeqStore);
        ZSTDMT_releaseCCtx((*job).cctxPool, cctx);
        /* report */
        ZSTD_pthread_mutex_lock(&mut (*job).job_mutex);
        if ZSTD_isError((*job).cSize) != 0 {
            debug_assert!(lastCBlockSize == 0);
        }
        (*job).cSize += lastCBlockSize;
        (*job).consumed = (*job).src.size; /* presumed completed */
        ZSTD_pthread_cond_signal(&mut (*job).job_cond);
        ZSTD_pthread_mutex_unlock(&mut (*job).job_mutex);
    }
}

/* ------------------------------------------ */
/* =====   Multi-threaded compression   ===== */
/* ------------------------------------------ */

#[repr(C)]
struct InBuff_t {
    prefix: Range, /* read-only non-owned prefix buffer */
    buffer: Buffer,
    filled: usize,
}

#[repr(C)]
struct RoundBuff_t {
    buffer: *mut u8, /* The round input buffer. */
    capacity: usize, /* The capacity of buffer. */
    pos: usize,      /* The position of the current inBuff in the round buffer. */
}

const kNullRoundBuff: RoundBuff_t = RoundBuff_t {
    buffer: core::ptr::null_mut(),
    capacity: 0,
    pos: 0,
};

const RSYNC_LENGTH: usize = 32;
const RSYNC_MIN_BLOCK_LOG: usize = ZSTD_BLOCKSIZELOG_MAX;
const RSYNC_MIN_BLOCK_SIZE: usize = 1 << RSYNC_MIN_BLOCK_LOG;

#[repr(C)]
struct RSyncState_t {
    hash: U64,
    hitMask: U64,
    primePower: U64,
}

#[repr(C)]
pub struct ZSTDMT_CCtx_s {
    factory: *mut POOL_ctx,
    jobs: *mut ZSTDMT_jobDescription,
    bufPool: *mut ZSTDMT_bufferPool,
    cctxPool: *mut ZSTDMT_CCtxPool,
    seqPool: *mut ZSTDMT_seqPool,
    params: ZSTD_CCtx_params,
    targetSectionSize: usize,
    targetPrefixSize: usize,
    jobReady: c_int, /* 1 => one job is already prepared, but pool has shortage of workers. */
    inBuff: InBuff_t,
    roundBuff: RoundBuff_t,
    serial: SerialState,
    rsync: RSyncState_t,
    jobIDMask: c_uint,
    doneJobID: c_uint,
    nextJobID: c_uint,
    frameEnded: c_uint,
    allJobsCompleted: c_uint,
    frameContentSize: c_ulonglong,
    consumed: c_ulonglong,
    produced: c_ulonglong,
    cMem: ZSTD_customMem,
    cdictLocal: *mut ZSTD_CDict,
    cdict: *const ZSTD_CDict,
    providedFactory: c_uint, /* bitfield: 1 */
}

pub type ZSTDMT_CCtx = ZSTDMT_CCtx_s;

unsafe fn ZSTDMT_freeJobsTable(
    jobTable: *mut ZSTDMT_jobDescription,
    nbJobs: U32,
    cMem: ZSTD_customMem,
) {
    if jobTable.is_null() {
        return;
    }
    let mut jobNb: U32 = 0;
    while jobNb < nbJobs {
        ZSTD_pthread_mutex_destroy(&mut (*jobTable.add(jobNb as usize)).job_mutex);
        ZSTD_pthread_cond_destroy(&mut (*jobTable.add(jobNb as usize)).job_cond);
        jobNb += 1;
    }
    zstd_custom_free(jobTable as *mut c_void, cMem);
}

/* ZSTDMT_createJobsTable()
 * update *nbJobsPtr to next power of 2 value, as size of table */
unsafe fn ZSTDMT_createJobsTable(
    nbJobsPtr: *mut U32,
    cMem: ZSTD_customMem,
) -> *mut ZSTDMT_jobDescription {
    let nbJobsLog2 = ZSTD_highbit32(*nbJobsPtr) + 1;
    let nbJobs: U32 = 1 << nbJobsLog2;
    let mut jobNb: U32;
    let jobTable = zstd_custom_calloc(
        nbJobs as usize * core::mem::size_of::<ZSTDMT_jobDescription>(),
        cMem,
    ) as *mut ZSTDMT_jobDescription;
    let mut initError: c_int = 0;
    if jobTable.is_null() {
        return core::ptr::null_mut();
    }
    *nbJobsPtr = nbJobs;
    jobNb = 0;
    while jobNb < nbJobs {
        initError |=
            ZSTD_pthread_mutex_init(&mut (*jobTable.add(jobNb as usize)).job_mutex, core::ptr::null());
        initError |=
            ZSTD_pthread_cond_init(&mut (*jobTable.add(jobNb as usize)).job_cond, core::ptr::null());
        jobNb += 1;
    }
    if initError != 0 {
        ZSTDMT_freeJobsTable(jobTable, nbJobs, cMem);
        return core::ptr::null_mut();
    }
    jobTable
}

unsafe fn ZSTDMT_expandJobsTable(mtctx: *mut ZSTDMT_CCtx, nbWorkers: U32) -> usize {
    let mut nbJobs: U32 = nbWorkers + 2;
    if nbJobs > (*mtctx).jobIDMask + 1 {
        /* need more job capacity */
        ZSTDMT_freeJobsTable((*mtctx).jobs, (*mtctx).jobIDMask + 1, (*mtctx).cMem);
        (*mtctx).jobIDMask = 0;
        (*mtctx).jobs = ZSTDMT_createJobsTable(&mut nbJobs, (*mtctx).cMem);
        if (*mtctx).jobs.is_null() {
            return error(code::MEMORY_ALLOCATION);
        }
        debug_assert!((nbJobs != 0) && ((nbJobs & (nbJobs - 1)) == 0));
        (*mtctx).jobIDMask = nbJobs - 1;
    }
    0
}

/* ZSTDMT_CCtxParam_setNbWorkers(): Internal use only */
unsafe fn ZSTDMT_CCtxParam_setNbWorkers(
    params: *mut ZSTD_CCtx_params,
    nbWorkers: c_uint,
) -> usize {
    ZSTD_CCtxParams_setParameter(params, ZSTD_c_nbWorkers, nbWorkers as c_int)
}

unsafe fn ZSTDMT_createCCtx_advanced_internal(
    mut nbWorkers: c_uint,
    cMem: ZSTD_customMem,
    pool: *mut ZSTD_threadPool,
) -> *mut ZSTDMT_CCtx {
    let mtctx: *mut ZSTDMT_CCtx;
    let mut nbJobs: U32 = nbWorkers + 2;
    let initError: c_int;

    if nbWorkers < 1 {
        return core::ptr::null_mut();
    }
    nbWorkers = min_usize(nbWorkers as usize, ZSTDMT_NBWORKERS_MAX as usize) as c_uint;
    if (!cMem.customAlloc.is_none()) ^ (!cMem.customFree.is_none()) {
        /* invalid custom allocator */
        return core::ptr::null_mut();
    }

    mtctx = zstd_custom_calloc(core::mem::size_of::<ZSTDMT_CCtx>(), cMem) as *mut ZSTDMT_CCtx;
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
    debug_assert!(nbJobs > 0);
    debug_assert!((nbJobs & (nbJobs - 1)) == 0);
    (*mtctx).jobIDMask = nbJobs - 1;
    (*mtctx).bufPool = ZSTDMT_createBufferPool(BUF_POOL_MAX_NB_BUFFERS(nbWorkers), cMem);
    (*mtctx).cctxPool = ZSTDMT_createCCtxPool(nbWorkers as c_int, cMem);
    (*mtctx).seqPool = ZSTDMT_createSeqPool(nbWorkers, cMem);
    initError = ZSTDMT_serialState_init(&mut (*mtctx).serial);
    (*mtctx).roundBuff = kNullRoundBuff;
    if ((*mtctx).factory.is_null() as c_int
        | (*mtctx).jobs.is_null() as c_int
        | (*mtctx).bufPool.is_null() as c_int
        | (*mtctx).cctxPool.is_null() as c_int
        | (*mtctx).seqPool.is_null() as c_int
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
    nbWorkers: c_uint,
    cMem: ZSTD_customMem,
    pool: *mut ZSTD_threadPool,
) -> *mut ZSTDMT_CCtx {
    /* ZSTD_MULTITHREAD not defined */
    let _ = nbWorkers;
    let _ = cMem;
    let _ = pool;
    core::ptr::null_mut()
}

/* ZSTDMT_releaseAllJobResources() :
 * note : ensure all workers are killed first ! */
unsafe fn ZSTDMT_releaseAllJobResources(mtctx: *mut ZSTDMT_CCtx) {
    let mut jobID: c_uint = 0;
    while jobID <= (*mtctx).jobIDMask {
        /* Copy the mutex/cond out */
        let mutex = (*(*mtctx).jobs.add(jobID as usize)).job_mutex;
        let cond = (*(*mtctx).jobs.add(jobID as usize)).job_cond;

        ZSTDMT_releaseBuffer((*mtctx).bufPool, (*(*mtctx).jobs.add(jobID as usize)).dstBuff);

        /* Clear the job description, but keep the mutex/cond */
        memset(
            (*mtctx).jobs.add(jobID as usize) as *mut c_void,
            0,
            core::mem::size_of::<ZSTDMT_jobDescription>(),
        );
        (*(*mtctx).jobs.add(jobID as usize)).job_mutex = mutex;
        (*(*mtctx).jobs.add(jobID as usize)).job_cond = cond;
        jobID += 1;
    }
    (*mtctx).inBuff.buffer = g_nullBuffer;
    (*mtctx).inBuff.filled = 0;
    (*mtctx).allJobsCompleted = 1;
}

unsafe fn ZSTDMT_waitForAllJobsCompleted(mtctx: *mut ZSTDMT_CCtx) {
    while (*mtctx).doneJobID < (*mtctx).nextJobID {
        let jobID = (*mtctx).doneJobID & (*mtctx).jobIDMask;
        ZSTD_pthread_mutex_lock(&mut (*(*mtctx).jobs.add(jobID as usize)).job_mutex);
        while (*(*mtctx).jobs.add(jobID as usize)).consumed
            < (*(*mtctx).jobs.add(jobID as usize)).src.size
        {
            ZSTD_pthread_cond_wait(
                &mut (*(*mtctx).jobs.add(jobID as usize)).job_cond,
                &mut (*(*mtctx).jobs.add(jobID as usize)).job_mutex,
            );
        }
        ZSTD_pthread_mutex_unlock(&mut (*(*mtctx).jobs.add(jobID as usize)).job_mutex);
        (*mtctx).doneJobID += 1;
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
    ZSTDMT_freeJobsTable((*mtctx).jobs, (*mtctx).jobIDMask + 1, (*mtctx).cMem);
    ZSTDMT_freeBufferPool((*mtctx).bufPool);
    ZSTDMT_freeCCtxPool((*mtctx).cctxPool);
    ZSTDMT_freeSeqPool((*mtctx).seqPool);
    ZSTDMT_serialState_free(&mut (*mtctx).serial);
    ZSTD_freeCDict((*mtctx).cdictLocal);
    if !(*mtctx).roundBuff.buffer.is_null() {
        zstd_custom_free((*mtctx).roundBuff.buffer as *mut c_void, (*mtctx).cMem);
    }
    zstd_custom_free(mtctx as *mut c_void, (*mtctx).cMem);
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
        + ((*mtctx).jobIDMask + 1) as usize * core::mem::size_of::<ZSTDMT_jobDescription>()
        + ZSTDMT_sizeof_CCtxPool((*mtctx).cctxPool)
        + ZSTDMT_sizeof_seqPool((*mtctx).seqPool)
        + ZSTD_sizeof_CDict((*mtctx).cdictLocal)
        + (*mtctx).roundBuff.capacity
}

/* ZSTDMT_resize() :
 * @return : error code if fails, 0 on success */
unsafe fn ZSTDMT_resize(mtctx: *mut ZSTDMT_CCtx, nbWorkers: c_uint) -> usize {
    if POOL_resize((*mtctx).factory, nbWorkers as usize) != 0 {
        return error(code::MEMORY_ALLOCATION);
    }
    {
        let _e = ZSTDMT_expandJobsTable(mtctx, nbWorkers);
        if err_is_error(_e) != 0 {
            return _e;
        }
    }
    (*mtctx).bufPool = ZSTDMT_expandBufferPool((*mtctx).bufPool, BUF_POOL_MAX_NB_BUFFERS(nbWorkers));
    if (*mtctx).bufPool.is_null() {
        return error(code::MEMORY_ALLOCATION);
    }
    (*mtctx).cctxPool = ZSTDMT_expandCCtxPool((*mtctx).cctxPool, nbWorkers as c_int);
    if (*mtctx).cctxPool.is_null() {
        return error(code::MEMORY_ALLOCATION);
    }
    (*mtctx).seqPool = ZSTDMT_expandSeqPool((*mtctx).seqPool, nbWorkers);
    if (*mtctx).seqPool.is_null() {
        return error(code::MEMORY_ALLOCATION);
    }
    ZSTDMT_CCtxParam_setNbWorkers(&mut (*mtctx).params, nbWorkers);
    0
}

/* ZSTDMT_updateCParams_whileCompressing() :
 *  Updates a selected set of compression parameters, remaining compatible with
 *  currently active frame. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_updateCParams_whileCompressing(
    mtctx: *mut ZSTDMT_CCtx,
    cctxParams: *const ZSTD_CCtx_params,
) {
    let saved_wlog = (*mtctx).params.cParams.windowLog; /* Do not modify windowLog while compressing */
    let compressionLevel = (*cctxParams).compressionLevel;
    (*mtctx).params.compressionLevel = compressionLevel;
    {
        let mut cParams = ZSTD_getCParamsFromCCtxParams(
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
 * tells how much data has been consumed (input) and produced (output) for
 * current frame. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_getFrameProgression(
    mtctx: *mut ZSTDMT_CCtx,
) -> ZSTD_frameProgression {
    let mut fps: ZSTD_frameProgression = ZSTD_frameProgression {
        ingested: 0,
        consumed: 0,
        produced: 0,
        flushed: 0,
        currentJobID: 0,
        nbActiveWorkers: 0,
    };
    fps.ingested = (*mtctx).consumed + (*mtctx).inBuff.filled as c_ulonglong;
    fps.consumed = (*mtctx).consumed;
    fps.produced = (*mtctx).produced;
    fps.flushed = (*mtctx).produced;
    fps.currentJobID = (*mtctx).nextJobID;
    fps.nbActiveWorkers = 0;
    {
        let mut jobNb: c_uint;
        let lastJobNb = (*mtctx).nextJobID + (*mtctx).jobReady as c_uint;
        debug_assert!((*mtctx).jobReady <= 1);
        jobNb = (*mtctx).doneJobID;
        while jobNb < lastJobNb {
            let wJobID = jobNb & (*mtctx).jobIDMask;
            let jobPtr = (*mtctx).jobs.add(wJobID as usize);
            ZSTD_pthread_mutex_lock(&mut (*jobPtr).job_mutex);
            {
                let cResult = (*jobPtr).cSize;
                let produced = if ZSTD_isError(cResult) != 0 { 0 } else { cResult };
                let flushed = if ZSTD_isError(cResult) != 0 {
                    0
                } else {
                    (*jobPtr).dstFlushed
                };
                debug_assert!(flushed <= produced);
                fps.ingested += (*jobPtr).src.size as c_ulonglong;
                fps.consumed += (*jobPtr).consumed as c_ulonglong;
                fps.produced += produced as c_ulonglong;
                fps.flushed += flushed as c_ulonglong;
                fps.nbActiveWorkers += ((*jobPtr).consumed < (*jobPtr).src.size) as c_uint;
            }
            ZSTD_pthread_mutex_unlock(&mut (*(*mtctx).jobs.add(wJobID as usize)).job_mutex);
            jobNb += 1;
        }
    }
    fps
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_toFlushNow(mtctx: *mut ZSTDMT_CCtx) -> usize {
    let mut toFlush: usize;
    let jobID = (*mtctx).doneJobID;
    debug_assert!(jobID <= (*mtctx).nextJobID);
    if jobID == (*mtctx).nextJobID {
        return 0; /* no active job => nothing to flush */
    }

    /* look into oldest non-fully-flushed job */
    {
        let wJobID = jobID & (*mtctx).jobIDMask;
        let jobPtr = (*mtctx).jobs.add(wJobID as usize);
        ZSTD_pthread_mutex_lock(&mut (*jobPtr).job_mutex);
        {
            let cResult = (*jobPtr).cSize;
            let produced = if ZSTD_isError(cResult) != 0 { 0 } else { cResult };
            let flushed = if ZSTD_isError(cResult) != 0 {
                0
            } else {
                (*jobPtr).dstFlushed
            };
            debug_assert!(flushed <= produced);
            debug_assert!((*jobPtr).consumed <= (*jobPtr).src.size);
            toFlush = produced - flushed;
            if toFlush == 0 {
                debug_assert!((*jobPtr).consumed < (*jobPtr).src.size);
            }
        }
        ZSTD_pthread_mutex_unlock(&mut (*(*mtctx).jobs.add(wJobID as usize)).job_mutex);
    }

    toFlush
}

/* ------------------------------------------ */
/* =====   Multi-threaded compression   ===== */
/* ------------------------------------------ */

unsafe fn ZSTDMT_computeTargetJobLog(params: *const ZSTD_CCtx_params) -> c_uint {
    let jobLog: c_uint;
    if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
        /* In Long Range Mode, the windowLog is typically oversized. */
        jobLog = max_u32(
            21,
            ZSTD_cycleLog((*params).cParams.chainLog, (*params).cParams.strategy) + 3,
        );
    } else {
        jobLog = max_u32(20, (*params).cParams.windowLog + 2);
    }
    min_usize(jobLog as usize, ZSTDMT_JOBLOG_MAX as usize) as c_uint
}

fn ZSTDMT_overlapLog_default(strat: ZSTD_strategy) -> c_int {
    match strat {
        ZSTD_btultra2 => 9,
        ZSTD_btultra | ZSTD_btopt => 8,
        ZSTD_btlazy2 | ZSTD_lazy2 => 7,
        // ZSTD_lazy | ZSTD_greedy | ZSTD_dfast | ZSTD_fast | default
        _ => 6,
    }
}

fn ZSTDMT_overlapLog(ovlog: c_int, strat: ZSTD_strategy) -> c_int {
    debug_assert!(0 <= ovlog && ovlog <= 9);
    if ovlog == 0 {
        return ZSTDMT_overlapLog_default(strat);
    }
    ovlog
}

unsafe fn ZSTDMT_computeOverlapSize(params: *const ZSTD_CCtx_params) -> usize {
    let overlapRLog: c_int = 9 - ZSTDMT_overlapLog((*params).overlapLog, (*params).cParams.strategy);
    let mut ovLog: c_int = if overlapRLog >= 8 {
        0
    } else {
        (*params).cParams.windowLog as c_int - overlapRLog
    };
    debug_assert!(0 <= overlapRLog && overlapRLog <= 8);
    if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
        /* In Long Range Mode, the windowLog is typically oversized. */
        ovLog = min_usize(
            (*params).cParams.windowLog as usize,
            (ZSTDMT_computeTargetJobLog(params) - 2) as usize,
        ) as c_int
            - overlapRLog;
    }
    debug_assert!(0 <= ovLog && ovLog <= ZSTD_WINDOWLOG_MAX);
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
    dict: *const c_void,
    dictSize: usize,
    dictContentType: ZSTD_dictContentType_e,
    cdict: *const ZSTD_CDict,
    mut params: ZSTD_CCtx_params,
    pledgedSrcSize: c_ulonglong,
) -> usize {
    /* params supposed partially fully validated at this point */
    debug_assert!(ZSTD_isError(ZSTD_checkCParams(params.cParams)) == 0);
    debug_assert!(!(!dict.is_null() && !cdict.is_null())); /* either dict or cdict, not both */

    /* init */
    if params.nbWorkers != (*mtctx).params.nbWorkers {
        let _e = ZSTDMT_resize(mtctx, params.nbWorkers as c_uint);
        if err_is_error(_e) != 0 {
            return _e;
        }
    }

    if params.jobSize != 0 && params.jobSize < ZSTDMT_JOBSIZE_MIN {
        params.jobSize = ZSTDMT_JOBSIZE_MIN;
    }
    if params.jobSize > ZSTDMT_JOBSIZE_MAX {
        params.jobSize = ZSTDMT_JOBSIZE_MAX;
    }

    if (*mtctx).allJobsCompleted == 0 {
        /* previous compression not correctly finished */
        ZSTDMT_waitForAllJobsCompleted(mtctx);
        ZSTDMT_releaseAllJobResources(mtctx);
        (*mtctx).allJobsCompleted = 1;
    }

    (*mtctx).params = copy_params(&params);
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
            return error(code::MEMORY_ALLOCATION);
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
    debug_assert!((*mtctx).targetSectionSize <= ZSTDMT_JOBSIZE_MAX);

    if params.rsyncable != 0 {
        /* Aim for the targetsectionSize as the average job size. */
        let jobSizeKB: U32 = ((*mtctx).targetSectionSize >> 10) as U32;
        let rsyncBits: U32 = {
            debug_assert!(jobSizeKB >= 1);
            ZSTD_highbit32(jobSizeKB) + 10
        };
        debug_assert!(rsyncBits >= (RSYNC_MIN_BLOCK_LOG + 2) as U32);
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
        let windowSize: usize = if (*mtctx).params.ldmParams.enableLdm == ZSTD_ps_enable {
            (1u32 << (*mtctx).params.cParams.windowLog) as usize
        } else {
            0
        };
        let nbSlackBuffers: usize = 2 + ((*mtctx).targetPrefixSize > 0) as usize;
        let slackSize: usize = (*mtctx).targetSectionSize * nbSlackBuffers;
        let nbWorkers: usize = max_usize((*mtctx).params.nbWorkers as usize, 1);
        let sectionsSize: usize = (*mtctx).targetSectionSize * nbWorkers;
        let capacity: usize = max_usize(windowSize, sectionsSize) + slackSize;
        if (*mtctx).roundBuff.capacity < capacity {
            if !(*mtctx).roundBuff.buffer.is_null() {
                zstd_custom_free((*mtctx).roundBuff.buffer as *mut c_void, (*mtctx).cMem);
            }
            (*mtctx).roundBuff.buffer = zstd_custom_malloc(capacity, (*mtctx).cMem) as *mut u8;
            if (*mtctx).roundBuff.buffer.is_null() {
                (*mtctx).roundBuff.capacity = 0;
                return error(code::MEMORY_ALLOCATION);
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
    (*mtctx).cdictLocal = core::ptr::null_mut();
    (*mtctx).cdict = core::ptr::null();
    if !dict.is_null() {
        if dictContentType == ZSTD_dct_rawContent {
            (*mtctx).inBuff.prefix.start = dict as *const u8 as *const c_void;
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
                return error(code::MEMORY_ALLOCATION);
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
        return error(code::MEMORY_ALLOCATION);
    }

    0
}

/* ZSTDMT_writeLastEmptyBlock()
 * Write a single empty block with an end-of-frame to finish a frame. */
unsafe fn ZSTDMT_writeLastEmptyBlock(job: *mut ZSTDMT_jobDescription) {
    debug_assert!((*job).lastJob == 1);
    debug_assert!((*job).src.size == 0);
    debug_assert!((*job).firstJob == 0);
    debug_assert!((*job).dstBuff.start.is_null());
    (*job).dstBuff = ZSTDMT_getBuffer((*job).bufPool);
    if (*job).dstBuff.start.is_null() {
        (*job).cSize = error(code::MEMORY_ALLOCATION);
        return;
    }
    debug_assert!((*job).dstBuff.capacity >= ZSTD_blockHeaderSize);
    (*job).src = kNullRange;
    (*job).cSize = ZSTD_writeLastEmptyBlock((*job).dstBuff.start, (*job).dstBuff.capacity);
    debug_assert!(ZSTD_isError((*job).cSize) == 0);
    debug_assert!((*job).consumed == 0);
}

unsafe fn ZSTDMT_createCompressionJob(
    mtctx: *mut ZSTDMT_CCtx,
    srcSize: usize,
    endOp: ZSTD_EndDirective,
) -> usize {
    let jobID = (*mtctx).nextJobID & (*mtctx).jobIDMask;
    let endFrame: c_int = (endOp == ZSTD_e_end) as c_int;

    if (*mtctx).nextJobID > (*mtctx).doneJobID + (*mtctx).jobIDMask {
        debug_assert!(
            ((*mtctx).nextJobID & (*mtctx).jobIDMask) == ((*mtctx).doneJobID & (*mtctx).jobIDMask)
        );
        return 0;
    }

    if (*mtctx).jobReady == 0 {
        let src = (*mtctx).inBuff.buffer.start as *const u8;
        (*(*mtctx).jobs.add(jobID as usize)).src.start = src as *const c_void;
        (*(*mtctx).jobs.add(jobID as usize)).src.size = srcSize;
        debug_assert!((*mtctx).inBuff.filled >= srcSize);
        (*(*mtctx).jobs.add(jobID as usize)).prefix = (*mtctx).inBuff.prefix;
        (*(*mtctx).jobs.add(jobID as usize)).consumed = 0;
        (*(*mtctx).jobs.add(jobID as usize)).cSize = 0;
        (*(*mtctx).jobs.add(jobID as usize)).params = copy_params(&(*mtctx).params);
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
        (*(*mtctx).jobs.add(jobID as usize)).firstJob = ((*mtctx).nextJobID == 0) as c_uint;
        (*(*mtctx).jobs.add(jobID as usize)).lastJob = endFrame as c_uint;
        (*(*mtctx).jobs.add(jobID as usize)).frameChecksumNeeded =
            ((*mtctx).params.fParams.checksumFlag != 0 && endFrame != 0 && (*mtctx).nextJobID > 0)
                as c_uint;
        (*(*mtctx).jobs.add(jobID as usize)).dstFlushed = 0;

        /* Update the round buffer pos and clear the input buffer to be reset */
        (*mtctx).roundBuff.pos += srcSize;
        (*mtctx).inBuff.buffer = g_nullBuffer;
        (*mtctx).inBuff.filled = 0;
        /* Set the prefix for next job */
        if endFrame == 0 {
            let newPrefixSize = min_usize(srcSize, (*mtctx).targetPrefixSize);
            (*mtctx).inBuff.prefix.start =
                (src.add(srcSize).sub(newPrefixSize)) as *const c_void;
            (*mtctx).inBuff.prefix.size = newPrefixSize;
        } else {
            /* endFrame==1 => no need for another input buffer */
            (*mtctx).inBuff.prefix = kNullRange;
            (*mtctx).frameEnded = endFrame as c_uint;
            if (*mtctx).nextJobID == 0 {
                /* single job exception : checksum already calculated within worker thread */
                (*mtctx).params.fParams.checksumFlag = 0;
            }
        }

        if (srcSize == 0) && ((*mtctx).nextJobID > 0)
        /*single job must also write frame header*/
        {
            debug_assert!(endOp == ZSTD_e_end);
            ZSTDMT_writeLastEmptyBlock((*mtctx).jobs.add(jobID as usize));
            (*mtctx).nextJobID += 1;
            return 0;
        }
    }

    if POOL_tryAdd(
        (*mtctx).factory,
        ZSTDMT_compressionJob as POOL_function,
        (*mtctx).jobs.add(jobID as usize) as *mut c_void,
    ) != 0
    {
        (*mtctx).nextJobID += 1;
        (*mtctx).jobReady = 0;
    } else {
        (*mtctx).jobReady = 1;
    }
    0
}

/* ZSTDMT_flushProduced() :
 *  flush whatever data has been produced but not yet flushed in current job. */
unsafe fn ZSTDMT_flushProduced(
    mtctx: *mut ZSTDMT_CCtx,
    output: *mut ZSTD_outBuffer,
    blockToFlush: c_uint,
    end: ZSTD_EndDirective,
) -> usize {
    let wJobID = (*mtctx).doneJobID & (*mtctx).jobIDMask;
    debug_assert!((*output).size >= (*output).pos);

    ZSTD_pthread_mutex_lock(&mut (*(*mtctx).jobs.add(wJobID as usize)).job_mutex);
    if blockToFlush != 0 && ((*mtctx).doneJobID < (*mtctx).nextJobID) {
        debug_assert!(
            (*(*mtctx).jobs.add(wJobID as usize)).dstFlushed
                <= (*(*mtctx).jobs.add(wJobID as usize)).cSize
        );
        while (*(*mtctx).jobs.add(wJobID as usize)).dstFlushed
            == (*(*mtctx).jobs.add(wJobID as usize)).cSize
        {
            /* nothing to flush */
            if (*(*mtctx).jobs.add(wJobID as usize)).consumed
                == (*(*mtctx).jobs.add(wJobID as usize)).src.size
            {
                break;
            }
            ZSTD_pthread_cond_wait(
                &mut (*(*mtctx).jobs.add(wJobID as usize)).job_cond,
                &mut (*(*mtctx).jobs.add(wJobID as usize)).job_mutex,
            );
        }
    }

    /* try to flush something */
    {
        let mut cSize = (*(*mtctx).jobs.add(wJobID as usize)).cSize; /* shared */
        let srcConsumed = (*(*mtctx).jobs.add(wJobID as usize)).consumed; /* shared */
        let srcSize = (*(*mtctx).jobs.add(wJobID as usize)).src.size; /* read-only */
        ZSTD_pthread_mutex_unlock(&mut (*(*mtctx).jobs.add(wJobID as usize)).job_mutex);
        if ZSTD_isError(cSize) != 0 {
            ZSTDMT_waitForAllJobsCompleted(mtctx);
            ZSTDMT_releaseAllJobResources(mtctx);
            return cSize;
        }
        /* add frame checksum if necessary (can only happen once) */
        debug_assert!(srcConsumed <= srcSize);
        if (srcConsumed == srcSize) /* job completed -> worker no longer active */
            && (*(*mtctx).jobs.add(wJobID as usize)).frameChecksumNeeded != 0
        {
            let checksum = ZSTD_XXH64_digest(&(*mtctx).serial.xxhState) as U32;
            mem_write_le32(
                ((*(*mtctx).jobs.add(wJobID as usize)).dstBuff.start as *mut c_char)
                    .add((*(*mtctx).jobs.add(wJobID as usize)).cSize)
                    as *mut c_void,
                checksum,
            );
            cSize += 4;
            (*(*mtctx).jobs.add(wJobID as usize)).cSize += 4; /* worker no longer active */
            (*(*mtctx).jobs.add(wJobID as usize)).frameChecksumNeeded = 0;
        }

        if cSize > 0 {
            /* compression is ongoing or completed */
            let toFlush = min_usize(
                cSize - (*(*mtctx).jobs.add(wJobID as usize)).dstFlushed,
                (*output).size - (*output).pos,
            );
            debug_assert!((*mtctx).doneJobID < (*mtctx).nextJobID);
            debug_assert!(cSize >= (*(*mtctx).jobs.add(wJobID as usize)).dstFlushed);
            debug_assert!(!(*(*mtctx).jobs.add(wJobID as usize)).dstBuff.start.is_null());
            if toFlush > 0 {
                memcpy(
                    ((*output).dst as *mut c_char).add((*output).pos) as *mut c_void,
                    ((*(*mtctx).jobs.add(wJobID as usize)).dstBuff.start as *const c_char)
                        .add((*(*mtctx).jobs.add(wJobID as usize)).dstFlushed)
                        as *const c_void,
                    toFlush,
                );
            }
            (*output).pos += toFlush;
            (*(*mtctx).jobs.add(wJobID as usize)).dstFlushed += toFlush; /* only used by mtctx */

            if (srcConsumed == srcSize) /* job is completed */
                && ((*(*mtctx).jobs.add(wJobID as usize)).dstFlushed == cSize)
            {
                /* output buffer fully flushed => free this job position */
                ZSTDMT_releaseBuffer(
                    (*mtctx).bufPool,
                    (*(*mtctx).jobs.add(wJobID as usize)).dstBuff,
                );
                (*(*mtctx).jobs.add(wJobID as usize)).dstBuff = g_nullBuffer;
                (*(*mtctx).jobs.add(wJobID as usize)).cSize = 0; /* considered "not started" */
                (*mtctx).consumed += srcSize as c_ulonglong;
                (*mtctx).produced += cSize as c_ulonglong;
                (*mtctx).doneJobID += 1;
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
    (*mtctx).allJobsCompleted = (*mtctx).frameEnded; /* all jobs entirely flushed */
    if end == ZSTD_e_end {
        return ((*mtctx).frameEnded == 0) as usize; /* is frame completed ? */
    }
    0 /* internal buffers fully flushed */
}

/**
 * Returns the range of data used by the earliest job that is not yet complete.
 */
unsafe fn ZSTDMT_getInputDataInUse(mtctx: *mut ZSTDMT_CCtx) -> Range {
    let firstJobID = (*mtctx).doneJobID;
    let lastJobID = (*mtctx).nextJobID;
    let mut jobID: c_uint;

    /* no need to check during first round */
    let roundBuffCapacity = (*mtctx).roundBuff.capacity;
    let nbJobs1stRoundMin = roundBuffCapacity / (*mtctx).targetSectionSize;
    if (lastJobID as usize) < nbJobs1stRoundMin {
        return kNullRange;
    }

    jobID = firstJobID;
    while jobID < lastJobID {
        let wJobID = jobID & (*mtctx).jobIDMask;
        let consumed: usize;

        ZSTD_pthread_mutex_lock(&mut (*(*mtctx).jobs.add(wJobID as usize)).job_mutex);
        consumed = (*(*mtctx).jobs.add(wJobID as usize)).consumed;
        ZSTD_pthread_mutex_unlock(&mut (*(*mtctx).jobs.add(wJobID as usize)).job_mutex);

        if consumed < (*(*mtctx).jobs.add(wJobID as usize)).src.size {
            let mut range = (*(*mtctx).jobs.add(wJobID as usize)).prefix;
            if range.size == 0 {
                /* Empty prefix */
                range = (*(*mtctx).jobs.add(wJobID as usize)).src;
            }
            debug_assert!(range.start <= (*(*mtctx).jobs.add(wJobID as usize)).src.start);
            return range;
        }
        jobID += 1;
    }
    kNullRange
}

/**
 * Returns non-zero iff buffer and range overlap.
 */
unsafe fn ZSTDMT_isOverlapped(buffer: Buffer, range: Range) -> c_int {
    let bufferStart = buffer.start as *const u8;
    let rangeStart = range.start as *const u8;

    if rangeStart.is_null() || bufferStart.is_null() {
        return 0;
    }

    {
        let bufferEnd = bufferStart.add(buffer.capacity);
        let rangeEnd = rangeStart.add(range.size);

        /* Empty ranges cannot overlap */
        if bufferStart == bufferEnd || rangeStart == rangeEnd {
            return 0;
        }

        (bufferStart < rangeEnd && rangeStart < bufferEnd) as c_int
    }
}

unsafe fn ZSTDMT_doesOverlapWindow(buffer: Buffer, window: ZSTD_window_t) -> c_int {
    let mut extDict: Range = Range {
        start: core::ptr::null(),
        size: 0,
    };
    let mut prefix: Range = Range {
        start: core::ptr::null(),
        size: 0,
    };

    extDict.start = window.dictBase.add(window.lowLimit as usize) as *const c_void;
    extDict.size = (window.dictLimit - window.lowLimit) as usize;

    prefix.start = window.base.add(window.dictLimit as usize) as *const c_void;
    prefix.size =
        window.nextSrc.offset_from(window.base.add(window.dictLimit as usize)) as usize;

    (ZSTDMT_isOverlapped(buffer, extDict) != 0 || ZSTDMT_isOverlapped(buffer, prefix) != 0) as c_int
}

unsafe fn ZSTDMT_waitForLdmComplete(mtctx: *mut ZSTDMT_CCtx, buffer: Buffer) {
    if (*mtctx).params.ldmParams.enableLdm == ZSTD_ps_enable {
        let mutex = &mut (*mtctx).serial.ldmWindowMutex;
        ZSTD_pthread_mutex_lock(mutex);
        while ZSTDMT_doesOverlapWindow(buffer, (*mtctx).serial.ldmWindow) != 0 {
            ZSTD_pthread_cond_wait(&mut (*mtctx).serial.ldmWindowCond, mutex);
        }
        ZSTD_pthread_mutex_unlock(mutex);
    }
}

/**
 * Attempts to set the inBuff to the next section to fill.
 */
unsafe fn ZSTDMT_tryGetInputRange(mtctx: *mut ZSTDMT_CCtx) -> c_int {
    let inUse = ZSTDMT_getInputDataInUse(mtctx);
    let spaceLeft = (*mtctx).roundBuff.capacity - (*mtctx).roundBuff.pos;
    let spaceNeeded = (*mtctx).targetSectionSize;
    let mut buffer: Buffer = Buffer {
        start: core::ptr::null_mut(),
        capacity: 0,
    };

    debug_assert!((*mtctx).inBuff.buffer.start.is_null());
    debug_assert!((*mtctx).roundBuff.capacity >= spaceNeeded);

    if spaceLeft < spaceNeeded {
        /* Simply copy the prefix to the beginning in that case. */
        let start = (*mtctx).roundBuff.buffer;
        let prefixSize = (*mtctx).inBuff.prefix.size;

        buffer.start = start as *mut c_void;
        buffer.capacity = prefixSize;
        if ZSTDMT_isOverlapped(buffer, inUse) != 0 {
            return 0;
        }
        ZSTDMT_waitForLdmComplete(mtctx, buffer);
        memmove(
            start as *mut c_void,
            (*mtctx).inBuff.prefix.start,
            prefixSize,
        );
        (*mtctx).inBuff.prefix.start = start as *const c_void;
        (*mtctx).roundBuff.pos = prefixSize;
    }
    buffer.start = (*mtctx).roundBuff.buffer.add((*mtctx).roundBuff.pos) as *mut c_void;
    buffer.capacity = spaceNeeded;

    if ZSTDMT_isOverlapped(buffer, inUse) != 0 {
        return 0;
    }
    debug_assert!(ZSTDMT_isOverlapped(buffer, (*mtctx).inBuff.prefix) == 0);

    ZSTDMT_waitForLdmComplete(mtctx, buffer);

    (*mtctx).inBuff.buffer = buffer;
    (*mtctx).inBuff.filled = 0;
    debug_assert!((*mtctx).roundBuff.pos + buffer.capacity <= (*mtctx).roundBuff.capacity);
    1
}

#[repr(C)]
struct SyncPoint {
    toLoad: usize, /* The number of bytes to load from the input. */
    flush: c_int,  /* Boolean : found a synchronization point. */
}

/**
 * Searches through the input for a synchronization point.
 */
unsafe fn findSynchronizationPoint(mtctx: *const ZSTDMT_CCtx, input: ZSTD_inBuffer) -> SyncPoint {
    let istart = (input.src as *const u8).add(input.pos);
    let primePower = (*mtctx).rsync.primePower;
    let hitMask = (*mtctx).rsync.hitMask;

    let mut syncPoint: SyncPoint = SyncPoint {
        toLoad: 0,
        flush: 0,
    };
    let mut hash: U64;
    let prev: *const u8;
    let mut pos: usize;

    syncPoint.toLoad = min_usize(
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
            prev = istart.add(pos).sub(RSYNC_LENGTH);
            hash = ZSTD_rollingHash_compute(prev as *const c_void, RSYNC_LENGTH);
        } else {
            debug_assert!((*mtctx).inBuff.filled >= RSYNC_LENGTH);
            prev = ((*mtctx).inBuff.buffer.start as *const u8)
                .add((*mtctx).inBuff.filled)
                .sub(RSYNC_LENGTH);
            hash = ZSTD_rollingHash_compute(prev.add(pos) as *const c_void, RSYNC_LENGTH - pos);
            hash = ZSTD_rollingHash_append(hash, istart as *const c_void, pos);
        }
    } else {
        debug_assert!((*mtctx).inBuff.filled >= RSYNC_MIN_BLOCK_SIZE);
        debug_assert!(RSYNC_MIN_BLOCK_SIZE >= RSYNC_LENGTH);
        pos = 0;
        prev = ((*mtctx).inBuff.buffer.start as *const u8)
            .add((*mtctx).inBuff.filled)
            .sub(RSYNC_LENGTH);
        hash = ZSTD_rollingHash_compute(prev as *const c_void, RSYNC_LENGTH);
        if (hash & hitMask) == hitMask {
            syncPoint.toLoad = 0;
            syncPoint.flush = 1;
            return syncPoint;
        }
    }
    /* Roll through the input. */
    while pos < syncPoint.toLoad {
        let toRemove: u8 = if pos < RSYNC_LENGTH {
            *prev.add(pos)
        } else {
            *istart.add(pos - RSYNC_LENGTH)
        };
        hash = ZSTD_rollingHash_rotate(hash, toRemove, *istart.add(pos), primePower);
        debug_assert!((*mtctx).inBuff.filled + pos >= RSYNC_MIN_BLOCK_SIZE);
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
    let mut hintInSize = (*mtctx).targetSectionSize - (*mtctx).inBuff.filled;
    if hintInSize == 0 {
        hintInSize = (*mtctx).targetSectionSize;
    }
    hintInSize
}

/** ZSTDMT_compressStream_generic() :
 *  internal use only - exposed to be invoked from zstd_compress.c */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_compressStream_generic(
    mtctx: *mut ZSTDMT_CCtx,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
    mut endOp: ZSTD_EndDirective,
) -> usize {
    let mut forwardInputProgress: c_uint = 0;
    debug_assert!((*output).pos <= (*output).size);
    debug_assert!((*input).pos <= (*input).size);

    if ((*mtctx).frameEnded != 0) && (endOp == ZSTD_e_continue) {
        /* current frame being ended. Only flush/end are allowed */
        return error(code::STAGE_WRONG);
    }

    /* fill input buffer */
    if ((*mtctx).jobReady == 0) && ((*input).size > (*input).pos) {
        /* support NULL input */
        if (*mtctx).inBuff.buffer.start.is_null() {
            debug_assert!((*mtctx).inBuff.filled == 0); /* Can't fill an empty buffer */
            if ZSTDMT_tryGetInputRange(mtctx) == 0 {
                /* only possible to fail if there are still jobs ongoing. */
                debug_assert!((*mtctx).doneJobID != (*mtctx).nextJobID);
            }
        }
        if !(*mtctx).inBuff.buffer.start.is_null() {
            let syncPoint = findSynchronizationPoint(mtctx, *input);
            if syncPoint.flush != 0 && endOp == ZSTD_e_continue {
                endOp = ZSTD_e_flush;
            }
            debug_assert!((*mtctx).inBuff.buffer.capacity >= (*mtctx).targetSectionSize);
            memcpy(
                ((*mtctx).inBuff.buffer.start as *mut c_char).add((*mtctx).inBuff.filled)
                    as *mut c_void,
                ((*input).src as *const c_char).add((*input).pos) as *const c_void,
                syncPoint.toLoad,
            );
            (*input).pos += syncPoint.toLoad;
            (*mtctx).inBuff.filled += syncPoint.toLoad;
            forwardInputProgress = (syncPoint.toLoad > 0) as c_uint;
        }
    }
    if ((*input).pos < (*input).size) && (endOp == ZSTD_e_end) {
        /* Can't end yet because the input is not fully consumed. */
        debug_assert!(
            (*mtctx).inBuff.filled == 0
                || (*mtctx).inBuff.filled == (*mtctx).targetSectionSize
                || (*mtctx).params.rsyncable != 0
        );
        endOp = ZSTD_e_flush;
    }

    if ((*mtctx).jobReady != 0)
        || ((*mtctx).inBuff.filled >= (*mtctx).targetSectionSize) /* filled enough */
        || ((endOp != ZSTD_e_continue) && ((*mtctx).inBuff.filled > 0)) /* something to flush */
        || ((endOp == ZSTD_e_end) && ((*mtctx).frameEnded == 0))
    {
        /* must finish the frame with a zero-size block */
        let jobSize = (*mtctx).inBuff.filled;
        debug_assert!((*mtctx).inBuff.filled <= (*mtctx).targetSectionSize);
        let _e = ZSTDMT_createCompressionJob(mtctx, jobSize, endOp);
        if err_is_error(_e) != 0 {
            return _e;
        }
    }

    /* check for potential compressed data ready to be flushed */
    {
        let remainingToFlush =
            ZSTDMT_flushProduced(mtctx, output, (forwardInputProgress == 0) as c_uint, endOp);
        if (*input).pos < (*input).size {
            return max_usize(remainingToFlush, 1); /* input not consumed */
        }
        remainingToFlush
    }
}

