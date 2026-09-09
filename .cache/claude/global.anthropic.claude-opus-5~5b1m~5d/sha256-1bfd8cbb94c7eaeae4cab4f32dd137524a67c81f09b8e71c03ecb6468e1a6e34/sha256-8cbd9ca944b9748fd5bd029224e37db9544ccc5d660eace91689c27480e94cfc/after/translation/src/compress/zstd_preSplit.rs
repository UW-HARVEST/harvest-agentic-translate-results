//! Translation of `compress/zstd_preSplit.c`
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::cmem::*;

/* ===== zstd_preSplit.h ===== */
/* `ZSTD_SLIPBLOCK_WORKSPACESIZE` is defined in
 * `crate::compress::zstd_compress_internal` (from zstd_compress_internal.h). */

const BLOCKSIZE_MIN: usize = 3500;
const THRESHOLD_PENALTY_RATE: c_int = 16;
const THRESHOLD_BASE: c_int = THRESHOLD_PENALTY_RATE - 2;
const THRESHOLD_PENALTY: c_int = 3;

const HASHLENGTH: usize = 2;
const HASHLOG_MAX: u32 = 10;
const HASHTABLESIZE: usize = 1 << HASHLOG_MAX;
const HASHMASK: usize = HASHTABLESIZE - 1;
const KNUTH: U32 = 0x9e3779b9;

/* for hashLog > 8, hash 2 bytes.
 * for hashLog == 8, just take the byte, no hashing.
 * The speed of this method relies on compile-time constant propagation */
#[inline(always)]
pub(crate) unsafe fn hash2(p: *const c_void, hashLog: c_uint) -> c_uint {
    if hashLog == 8 {
        return *(p as *const BYTE) as U32;
    }
    ((MEM_read16(p) as U32).wrapping_mul(KNUTH)) >> (32 - hashLog)
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Fingerprint {
    pub events: [c_uint; HASHTABLESIZE],
    pub nbEvents: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FPStats {
    pub pastEvents: Fingerprint,
    pub newEvents: Fingerprint,
}

pub(crate) unsafe fn initStats(fpstats: *mut FPStats) {
    ZSTD_memset(
        fpstats as *mut c_void,
        0,
        core::mem::size_of::<FPStats>(),
    );
}

#[inline(always)]
pub(crate) unsafe fn addEvents_generic(
    fp: *mut Fingerprint,
    src: *const c_void,
    srcSize: usize,
    samplingRate: usize,
    hashLog: c_uint,
) {
    let p = src as *const c_char;
    let limit: usize = srcSize - HASHLENGTH + 1;
    let mut n: usize;
    n = 0;
    while n < limit {
        *(*fp)
            .events
            .as_mut_ptr()
            .add(hash2(p.add(n) as *const c_void, hashLog) as usize) += 1;
        n += samplingRate;
    }
    (*fp).nbEvents = (*fp).nbEvents.wrapping_add(limit / samplingRate);
}

#[inline(always)]
pub(crate) unsafe fn recordFingerprint_generic(
    fp: *mut Fingerprint,
    src: *const c_void,
    srcSize: usize,
    samplingRate: usize,
    hashLog: c_uint,
) {
    ZSTD_memset(
        fp as *mut c_void,
        0,
        core::mem::size_of::<c_uint>() * (1usize << hashLog),
    );
    (*fp).nbEvents = 0;
    addEvents_generic(fp, src, srcSize, samplingRate, hashLog);
}

pub type RecordEvents_f = unsafe extern "C" fn(*mut Fingerprint, *const c_void, usize);

/* ZSTD_GEN_RECORD_FINGERPRINT(1, 10) */
pub(crate) unsafe extern "C" fn ZSTD_recordFingerprint_1(
    fp: *mut Fingerprint,
    src: *const c_void,
    srcSize: usize,
) {
    recordFingerprint_generic(fp, src, srcSize, 1, 10);
}

/* ZSTD_GEN_RECORD_FINGERPRINT(5, 10) */
pub(crate) unsafe extern "C" fn ZSTD_recordFingerprint_5(
    fp: *mut Fingerprint,
    src: *const c_void,
    srcSize: usize,
) {
    recordFingerprint_generic(fp, src, srcSize, 5, 10);
}

/* ZSTD_GEN_RECORD_FINGERPRINT(11, 9) */
pub(crate) unsafe extern "C" fn ZSTD_recordFingerprint_11(
    fp: *mut Fingerprint,
    src: *const c_void,
    srcSize: usize,
) {
    recordFingerprint_generic(fp, src, srcSize, 11, 9);
}

/* ZSTD_GEN_RECORD_FINGERPRINT(43, 8) */
pub(crate) unsafe extern "C" fn ZSTD_recordFingerprint_43(
    fp: *mut Fingerprint,
    src: *const c_void,
    srcSize: usize,
) {
    recordFingerprint_generic(fp, src, srcSize, 43, 8);
}

pub(crate) fn abs64(s64: S64) -> U64 {
    (if s64 < 0 { s64.wrapping_neg() } else { s64 }) as U64
}

pub(crate) unsafe fn fpDistance(
    fp1: *const Fingerprint,
    fp2: *const Fingerprint,
    hashLog: c_uint,
) -> U64 {
    let mut distance: U64 = 0;
    let mut n: usize;
    n = 0;
    while n < (1usize << hashLog) {
        distance = distance.wrapping_add(abs64(
            ((*fp1).events[n] as S64).wrapping_mul((*fp2).nbEvents as S64).wrapping_sub(
                ((*fp2).events[n] as S64).wrapping_mul((*fp1).nbEvents as S64),
            ),
        ));
        n += 1;
    }
    distance
}

/* Compare newEvents with pastEvents
 * return 1 when considered "too different"
 */
pub(crate) unsafe fn compareFingerprints(
    reference: *const Fingerprint,
    newfp: *const Fingerprint,
    penalty: c_int,
    hashLog: c_uint,
) -> c_int {
    {
        let p50: U64 = ((*reference).nbEvents as U64).wrapping_mul((*newfp).nbEvents as U64);
        let deviation: U64 = fpDistance(reference, newfp, hashLog);
        let threshold: U64 = p50
            .wrapping_mul((THRESHOLD_BASE + penalty) as U64)
            / THRESHOLD_PENALTY_RATE as U64;
        return (deviation >= threshold) as c_int;
    }
}

pub(crate) unsafe fn mergeEvents(acc: *mut Fingerprint, newfp: *const Fingerprint) {
    let mut n: usize;
    n = 0;
    while n < HASHTABLESIZE {
        (*acc).events[n] = (*acc).events[n].wrapping_add((*newfp).events[n]);
        n += 1;
    }
    (*acc).nbEvents = (*acc).nbEvents.wrapping_add((*newfp).nbEvents);
}

pub(crate) unsafe fn flushEvents(fpstats: *mut FPStats) {
    let mut n: usize;
    n = 0;
    while n < HASHTABLESIZE {
        (*fpstats).pastEvents.events[n] = (*fpstats).newEvents.events[n];
        n += 1;
    }
    (*fpstats).pastEvents.nbEvents = (*fpstats).newEvents.nbEvents;
    ZSTD_memset(
        &mut (*fpstats).newEvents as *mut Fingerprint as *mut c_void,
        0,
        core::mem::size_of::<Fingerprint>(),
    );
}

pub(crate) unsafe fn removeEvents(acc: *mut Fingerprint, slice: *const Fingerprint) {
    let mut n: usize;
    n = 0;
    while n < HASHTABLESIZE {
        (*acc).events[n] = (*acc).events[n].wrapping_sub((*slice).events[n]);
        n += 1;
    }
    (*acc).nbEvents = (*acc).nbEvents.wrapping_sub((*slice).nbEvents);
}

const CHUNKSIZE: usize = 8 << 10;

static records_fs: [RecordEvents_f; 4] = [
    ZSTD_recordFingerprint_43,
    ZSTD_recordFingerprint_11,
    ZSTD_recordFingerprint_5,
    ZSTD_recordFingerprint_1,
];
static hashParams: [c_uint; 4] = [8, 9, 10, 10];

pub(crate) unsafe fn ZSTD_splitBlock_byChunks(
    blockStart: *const c_void,
    blockSize: usize,
    level: c_int,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let record_f: RecordEvents_f = records_fs[level as usize];
    let fpstats = workspace as *mut FPStats;
    let p = blockStart as *const c_char;
    let mut penalty: c_int = THRESHOLD_PENALTY;
    let mut pos: usize = 0;

    initStats(fpstats);
    record_f(
        &mut (*fpstats).pastEvents,
        p as *const c_void,
        CHUNKSIZE,
    );
    pos = CHUNKSIZE;
    while pos <= blockSize - CHUNKSIZE {
        record_f(
            &mut (*fpstats).newEvents,
            p.add(pos) as *const c_void,
            CHUNKSIZE,
        );
        if compareFingerprints(
            &(*fpstats).pastEvents,
            &(*fpstats).newEvents,
            penalty,
            hashParams[level as usize],
        ) != 0
        {
            return pos;
        } else {
            mergeEvents(&mut (*fpstats).pastEvents, &(*fpstats).newEvents);
            if penalty > 0 {
                penalty -= 1;
            }
        }
        pos += CHUNKSIZE;
    }
    blockSize
}

const SEGMENT_SIZE: usize = 512;

/* ZSTD_splitBlock_fromBorders(): very fast strategy :
 * compare fingerprint from beginning and end of the block,
 * derive from their difference if it's preferable to split in the middle,
 * repeat the process a second time, for finer grained decision.
 * 3 times did not brought improvements, so I stopped at 2.
 * Benefits are good enough for a cheap heuristic.
 * More accurate splitting saves more, but speed impact is also more perceptible.
 * For better accuracy, use more elaborate variant *_byChunks.
 */
pub(crate) unsafe fn ZSTD_splitBlock_fromBorders(
    blockStart: *const c_void,
    blockSize: usize,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let fpstats = workspace as *mut FPStats;
    let middleEvents = (workspace as *mut c_char).add(512 * core::mem::size_of::<c_uint>())
        as *mut c_void as *mut Fingerprint;

    initStats(fpstats);
    crate::compress::hist::HIST_add(
        (*fpstats).pastEvents.events.as_mut_ptr(),
        blockStart,
        SEGMENT_SIZE,
    );
    crate::compress::hist::HIST_add(
        (*fpstats).newEvents.events.as_mut_ptr(),
        (blockStart as *const c_char).add(blockSize - SEGMENT_SIZE) as *const c_void,
        SEGMENT_SIZE,
    );
    (*fpstats).newEvents.nbEvents = SEGMENT_SIZE;
    (*fpstats).pastEvents.nbEvents = (*fpstats).newEvents.nbEvents;
    if compareFingerprints(&(*fpstats).pastEvents, &(*fpstats).newEvents, 0, 8) == 0 {
        return blockSize;
    }

    crate::compress::hist::HIST_add(
        (*middleEvents).events.as_mut_ptr(),
        (blockStart as *const c_char).add(blockSize / 2 - SEGMENT_SIZE / 2) as *const c_void,
        SEGMENT_SIZE,
    );
    (*middleEvents).nbEvents = SEGMENT_SIZE;
    {
        let distFromBegin: U64 = fpDistance(&(*fpstats).pastEvents, middleEvents, 8);
        let distFromEnd: U64 = fpDistance(&(*fpstats).newEvents, middleEvents, 8);
        let minDistance: U64 = (SEGMENT_SIZE * SEGMENT_SIZE / 3) as U64;
        if abs64((distFromBegin as S64).wrapping_sub(distFromEnd as S64)) < minDistance {
            return 64 * (1 << 10);
        }
        return if distFromBegin > distFromEnd {
            32 * (1 << 10)
        } else {
            96 * (1 << 10)
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_splitBlock(
    blockStart: *const c_void,
    blockSize: usize,
    level: c_int,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    if level == 0 {
        return ZSTD_splitBlock_fromBorders(blockStart, blockSize, workspace, wkspSize);
    }
    /* level >= 1*/
    ZSTD_splitBlock_byChunks(blockStart, blockSize, level - 1, workspace, wkspSize)
}
