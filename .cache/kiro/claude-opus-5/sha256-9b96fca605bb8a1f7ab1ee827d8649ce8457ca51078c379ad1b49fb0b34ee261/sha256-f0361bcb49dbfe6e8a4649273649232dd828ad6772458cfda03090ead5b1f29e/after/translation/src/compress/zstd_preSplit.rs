//! Translation of `compress/zstd_preSplit.c` (pre-sequences block splitter).
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use crate::common::mem::*;
use crate::compress::hist::HIST_add;

use core::ffi::{c_int, c_uint, c_void};

pub const ZSTD_SLIPBLOCK_WORKSPACESIZE: size_t = 8208;

const BLOCKSIZE_MIN: size_t = 3500;
const THRESHOLD_PENALTY_RATE: U64 = 16;
const THRESHOLD_BASE: U64 = THRESHOLD_PENALTY_RATE - 2;
const THRESHOLD_PENALTY: c_int = 3;

const HASHLENGTH: size_t = 2;
const HASHLOG_MAX: c_uint = 10;
const HASHTABLESIZE: usize = 1 << HASHLOG_MAX;
const HASHMASK: usize = HASHTABLESIZE - 1;
const KNUTH: U32 = 0x9e3779b9;

/* for hashLog > 8, hash 2 bytes.
 * for hashLog == 8, just take the byte, no hashing.
 * The speed of this method relies on compile-time constant propagation */
unsafe fn hash2(p: *const c_void, hashLog: c_uint) -> c_uint {
    if hashLog == 8 {
        return *(p as *const BYTE).add(0) as U32;
    }
    ((MEM_read16(p as *const u8) as U32).wrapping_mul(KNUTH)) >> (32 - hashLog)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Fingerprint {
    events: [c_uint; HASHTABLESIZE],
    nbEvents: size_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct FPStats {
    pastEvents: Fingerprint,
    newEvents: Fingerprint,
}

unsafe fn initStats(fpstats: *mut FPStats) {
    ZSTD_memset(
        fpstats as *mut u8,
        0,
        core::mem::size_of::<FPStats>(),
    );
}

unsafe fn addEvents_generic(
    fp: *mut Fingerprint,
    src: *const c_void,
    srcSize: size_t,
    samplingRate: size_t,
    hashLog: c_uint,
) {
    let p: *const c_char_ = src as *const c_char_;
    let limit: size_t = srcSize - HASHLENGTH + 1;
    let mut n: size_t = 0;
    while n < limit {
        let idx = hash2(p.add(n) as *const c_void, hashLog) as usize;
        (*fp).events[idx] += 1;
        n += samplingRate;
    }
    (*fp).nbEvents += limit / samplingRate;
}

unsafe fn recordFingerprint_generic(
    fp: *mut Fingerprint,
    src: *const c_void,
    srcSize: size_t,
    samplingRate: size_t,
    hashLog: c_uint,
) {
    ZSTD_memset(
        fp as *mut u8,
        0,
        core::mem::size_of::<c_uint>() * ((1 as size_t) << hashLog),
    );
    (*fp).nbEvents = 0;
    addEvents_generic(fp, src, srcSize, samplingRate, hashLog);
}

type RecordEvents_f = unsafe fn(fp: *mut Fingerprint, src: *const c_void, srcSize: size_t);

unsafe fn ZSTD_recordFingerprint_1(fp: *mut Fingerprint, src: *const c_void, srcSize: size_t) {
    recordFingerprint_generic(fp, src, srcSize, 1, 10);
}
unsafe fn ZSTD_recordFingerprint_5(fp: *mut Fingerprint, src: *const c_void, srcSize: size_t) {
    recordFingerprint_generic(fp, src, srcSize, 5, 10);
}
unsafe fn ZSTD_recordFingerprint_11(fp: *mut Fingerprint, src: *const c_void, srcSize: size_t) {
    recordFingerprint_generic(fp, src, srcSize, 11, 9);
}
unsafe fn ZSTD_recordFingerprint_43(fp: *mut Fingerprint, src: *const c_void, srcSize: size_t) {
    recordFingerprint_generic(fp, src, srcSize, 43, 8);
}

unsafe fn abs64(s64: S64) -> U64 {
    (if s64 < 0 { s64.wrapping_neg() } else { s64 }) as U64
}

unsafe fn fpDistance(fp1: *const Fingerprint, fp2: *const Fingerprint, hashLog: c_uint) -> U64 {
    let mut distance: U64 = 0;
    let mut n: size_t = 0;
    while n < ((1 as size_t) << hashLog) {
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
unsafe fn compareFingerprints(
    reference: *const Fingerprint,
    newfp: *const Fingerprint,
    penalty: c_int,
    hashLog: c_uint,
) -> c_int {
    {
        let p50: U64 = (*reference).nbEvents as U64 * (*newfp).nbEvents as U64;
        let deviation: U64 = fpDistance(reference, newfp, hashLog);
        let threshold: U64 =
            p50 * (THRESHOLD_BASE + penalty as U64) / THRESHOLD_PENALTY_RATE;
        (deviation >= threshold) as c_int
    }
}

unsafe fn mergeEvents(acc: *mut Fingerprint, newfp: *const Fingerprint) {
    let mut n: size_t = 0;
    while n < HASHTABLESIZE {
        (*acc).events[n] = (*acc).events[n].wrapping_add((*newfp).events[n]);
        n += 1;
    }
    (*acc).nbEvents += (*newfp).nbEvents;
}

unsafe fn flushEvents(fpstats: *mut FPStats) {
    let mut n: size_t = 0;
    while n < HASHTABLESIZE {
        (*fpstats).pastEvents.events[n] = (*fpstats).newEvents.events[n];
        n += 1;
    }
    (*fpstats).pastEvents.nbEvents = (*fpstats).newEvents.nbEvents;
    ZSTD_memset(
        &mut (*fpstats).newEvents as *mut Fingerprint as *mut u8,
        0,
        core::mem::size_of::<Fingerprint>(),
    );
}

unsafe fn removeEvents(acc: *mut Fingerprint, slice: *const Fingerprint) {
    let mut n: size_t = 0;
    while n < HASHTABLESIZE {
        (*acc).events[n] = (*acc).events[n].wrapping_sub((*slice).events[n]);
        n += 1;
    }
    (*acc).nbEvents -= (*slice).nbEvents;
}

const CHUNKSIZE: size_t = 8 << 10;
unsafe fn ZSTD_splitBlock_byChunks(
    blockStart: *const c_void,
    blockSize: size_t,
    level: c_int,
    workspace: *mut c_void,
    wkspSize: size_t,
) -> size_t {
    static records_fs: [RecordEvents_f; 4] = [
        ZSTD_recordFingerprint_43,
        ZSTD_recordFingerprint_11,
        ZSTD_recordFingerprint_5,
        ZSTD_recordFingerprint_1,
    ];
    static hashParams: [c_uint; 4] = [8, 9, 10, 10];
    let record_f: RecordEvents_f = records_fs[level as usize];
    let fpstats: *mut FPStats = workspace as *mut FPStats;
    let p: *const c_char_ = blockStart as *const c_char_;
    let mut penalty: c_int = THRESHOLD_PENALTY;
    let mut pos: size_t;
    let _ = wkspSize;

    initStats(fpstats);
    record_f(&mut (*fpstats).pastEvents, p as *const c_void, CHUNKSIZE);
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
    let _ = flushEvents;
    let _ = removeEvents;
    blockSize
}

/* ZSTD_splitBlock_fromBorders(): very fast strategy */
const SEGMENT_SIZE: size_t = 512;
unsafe fn ZSTD_splitBlock_fromBorders(
    blockStart: *const c_void,
    blockSize: size_t,
    workspace: *mut c_void,
    wkspSize: size_t,
) -> size_t {
    let fpstats: *mut FPStats = workspace as *mut FPStats;
    let middleEvents: *mut Fingerprint = (workspace as *mut c_char_)
        .add(512 * core::mem::size_of::<c_uint>())
        as *mut c_void as *mut Fingerprint;
    let _ = wkspSize;

    initStats(fpstats);
    HIST_add(
        (*fpstats).pastEvents.events.as_mut_ptr(),
        blockStart,
        SEGMENT_SIZE,
    );
    HIST_add(
        (*fpstats).newEvents.events.as_mut_ptr(),
        (blockStart as *const c_char_).add(blockSize - SEGMENT_SIZE) as *const c_void,
        SEGMENT_SIZE,
    );
    (*fpstats).pastEvents.nbEvents = SEGMENT_SIZE;
    (*fpstats).newEvents.nbEvents = SEGMENT_SIZE;
    if compareFingerprints(&(*fpstats).pastEvents, &(*fpstats).newEvents, 0, 8) == 0 {
        return blockSize;
    }

    HIST_add(
        (*middleEvents).events.as_mut_ptr(),
        (blockStart as *const c_char_).add(blockSize / 2 - SEGMENT_SIZE / 2) as *const c_void,
        SEGMENT_SIZE,
    );
    (*middleEvents).nbEvents = SEGMENT_SIZE;
    {
        let distFromBegin: U64 = fpDistance(&(*fpstats).pastEvents, middleEvents, 8);
        let distFromEnd: U64 = fpDistance(&(*fpstats).newEvents, middleEvents, 8);
        let minDistance: U64 = (SEGMENT_SIZE * SEGMENT_SIZE / 3) as U64;
        if abs64((distFromBegin as S64).wrapping_sub(distFromEnd as S64)) < minDistance {
            return 64 << 10;
        }
        if distFromBegin > distFromEnd {
            32 << 10
        } else {
            96 << 10
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_splitBlock(
    blockStart: *const c_void,
    blockSize: size_t,
    level: c_int,
    workspace: *mut c_void,
    wkspSize: size_t,
) -> size_t {
    if level == 0 {
        return ZSTD_splitBlock_fromBorders(blockStart, blockSize, workspace, wkspSize);
    }
    /* level >= 1*/
    ZSTD_splitBlock_byChunks(blockStart, blockSize, level - 1, workspace, wkspSize)
}

/* local alias to avoid depending on libc c_char signedness */
#[allow(non_camel_case_types)]
type c_char_ = u8;
