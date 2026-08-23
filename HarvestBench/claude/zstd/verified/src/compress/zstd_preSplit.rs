//! Translation of `compress/zstd_preSplit.c`
#![allow(dead_code)]

use crate::common::mem::*;
use crate::libc::*;
use core::ffi::{c_char, c_int, c_uint, c_void};

pub const ZSTD_SLIPBLOCK_WORKSPACESIZE: usize = 8208;

const BLOCKSIZE_MIN: usize = 3500;
const THRESHOLD_PENALTY_RATE: u64 = 16;
const THRESHOLD_BASE: u64 = THRESHOLD_PENALTY_RATE - 2;
const THRESHOLD_PENALTY: c_int = 3;

const HASHLENGTH: usize = 2;
const HASHLOG_MAX: u32 = 10;
const HASHTABLESIZE: usize = 1 << HASHLOG_MAX;
const HASHMASK: usize = HASHTABLESIZE - 1;
const KNUTH: U32 = 0x9e3779b9;

extern "C" {
    /* compress/hist.c */
    fn HIST_add(count: *mut c_uint, src: *const c_void, srcSize: usize);
}

#[inline(always)]
unsafe fn hash2(p: *const c_void, hashLog: c_uint) -> c_uint {
    if hashLog == 8 {
        return *(p as *const BYTE) as U32;
    }
    ((MEM_read16(p) as U32).wrapping_mul(KNUTH)) >> (32 - hashLog)
}

#[repr(C)]
#[derive(Copy, Clone)]
struct Fingerprint {
    events: [c_uint; HASHTABLESIZE],
    nbEvents: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FPStats {
    pastEvents: Fingerprint,
    newEvents: Fingerprint,
}

unsafe fn initStats(fpstats: *mut FPStats) {
    ZSTD_memset(
        fpstats as *mut c_void,
        0,
        core::mem::size_of::<FPStats>(),
    );
}

#[inline(always)]
unsafe fn addEvents_generic(
    fp: *mut Fingerprint,
    src: *const c_void,
    srcSize: usize,
    samplingRate: usize,
    hashLog: c_uint,
) {
    let p = src as *const c_char;
    let limit = srcSize - HASHLENGTH + 1;
    let mut n: usize = 0;
    while n < limit {
        let idx = hash2(p.add(n) as *const c_void, hashLog) as usize;
        (*fp).events[idx] += 1;
        n += samplingRate;
    }
    (*fp).nbEvents += limit / samplingRate;
}

#[inline(always)]
unsafe fn recordFingerprint_generic(
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

type RecordEvents_f = unsafe fn(*mut Fingerprint, *const c_void, usize);

unsafe fn ZSTD_recordFingerprint_1(fp: *mut Fingerprint, src: *const c_void, srcSize: usize) {
    recordFingerprint_generic(fp, src, srcSize, 1, 10);
}

unsafe fn ZSTD_recordFingerprint_5(fp: *mut Fingerprint, src: *const c_void, srcSize: usize) {
    recordFingerprint_generic(fp, src, srcSize, 5, 10);
}

unsafe fn ZSTD_recordFingerprint_11(fp: *mut Fingerprint, src: *const c_void, srcSize: usize) {
    recordFingerprint_generic(fp, src, srcSize, 11, 9);
}

unsafe fn ZSTD_recordFingerprint_43(fp: *mut Fingerprint, src: *const c_void, srcSize: usize) {
    recordFingerprint_generic(fp, src, srcSize, 43, 8);
}

fn abs64(s64: S64) -> U64 {
    (if s64 < 0 { s64.wrapping_neg() } else { s64 }) as U64
}

unsafe fn fpDistance(
    fp1: *const Fingerprint,
    fp2: *const Fingerprint,
    hashLog: c_uint,
) -> U64 {
    let mut distance: U64 = 0;
    let mut n: usize = 0;
    while n < (1usize << hashLog) {
        distance = distance.wrapping_add(abs64(
            ((*fp1).events[n] as S64)
                .wrapping_mul((*fp2).nbEvents as S64)
                .wrapping_sub(((*fp2).events[n] as S64).wrapping_mul((*fp1).nbEvents as S64)),
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
        let p50: U64 = ((*reference).nbEvents as U64).wrapping_mul((*newfp).nbEvents as U64);
        let deviation: U64 = fpDistance(reference, newfp, hashLog);
        let threshold: U64 = p50
            .wrapping_mul(THRESHOLD_BASE.wrapping_add(penalty as U64))
            / THRESHOLD_PENALTY_RATE;
        (deviation >= threshold) as c_int
    }
}

unsafe fn mergeEvents(acc: *mut Fingerprint, newfp: *const Fingerprint) {
    let mut n: usize = 0;
    while n < HASHTABLESIZE {
        (*acc).events[n] = (*acc).events[n].wrapping_add((*newfp).events[n]);
        n += 1;
    }
    (*acc).nbEvents = (*acc).nbEvents.wrapping_add((*newfp).nbEvents);
}

unsafe fn flushEvents(fpstats: *mut FPStats) {
    let mut n: usize = 0;
    while n < HASHTABLESIZE {
        (*fpstats).pastEvents.events[n] = (*fpstats).newEvents.events[n];
        n += 1;
    }
    (*fpstats).pastEvents.nbEvents = (*fpstats).newEvents.nbEvents;
    ZSTD_memset(
        core::ptr::addr_of_mut!((*fpstats).newEvents) as *mut c_void,
        0,
        core::mem::size_of::<Fingerprint>(),
    );
}

unsafe fn removeEvents(acc: *mut Fingerprint, slice: *const Fingerprint) {
    let mut n: usize = 0;
    while n < HASHTABLESIZE {
        (*acc).events[n] = (*acc).events[n].wrapping_sub((*slice).events[n]);
        n += 1;
    }
    (*acc).nbEvents = (*acc).nbEvents.wrapping_sub((*slice).nbEvents);
}

const CHUNKSIZE: usize = 8 << 10;

unsafe fn ZSTD_splitBlock_byChunks(
    blockStart: *const c_void,
    blockSize: usize,
    level: c_int,
    workspace: *mut c_void,
    _wkspSize: usize,
) -> usize {
    static records_fs: [RecordEvents_f; 4] = [
        ZSTD_recordFingerprint_43,
        ZSTD_recordFingerprint_11,
        ZSTD_recordFingerprint_5,
        ZSTD_recordFingerprint_1,
    ];
    static hashParams: [c_uint; 4] = [8, 9, 10, 10];
    let record_f = records_fs[level as usize];
    let fpstats = workspace as *mut FPStats;
    let p = blockStart as *const c_char;
    let mut penalty: c_int = THRESHOLD_PENALTY;
    let mut pos: usize = 0;

    initStats(fpstats);
    record_f(
        core::ptr::addr_of_mut!((*fpstats).pastEvents),
        p as *const c_void,
        CHUNKSIZE,
    );
    pos = CHUNKSIZE;
    while pos <= blockSize - CHUNKSIZE {
        record_f(
            core::ptr::addr_of_mut!((*fpstats).newEvents),
            p.add(pos) as *const c_void,
            CHUNKSIZE,
        );
        if compareFingerprints(
            core::ptr::addr_of!((*fpstats).pastEvents),
            core::ptr::addr_of!((*fpstats).newEvents),
            penalty,
            hashParams[level as usize],
        ) != 0
        {
            return pos;
        } else {
            mergeEvents(
                core::ptr::addr_of_mut!((*fpstats).pastEvents),
                core::ptr::addr_of!((*fpstats).newEvents),
            );
            if penalty > 0 {
                penalty -= 1;
            }
        }
        pos += CHUNKSIZE;
    }
    blockSize
}

const SEGMENT_SIZE: usize = 512;

unsafe fn ZSTD_splitBlock_fromBorders(
    blockStart: *const c_void,
    blockSize: usize,
    workspace: *mut c_void,
    _wkspSize: usize,
) -> usize {
    let fpstats = workspace as *mut FPStats;
    let middleEvents = (workspace as *mut c_char)
        .add(512 * core::mem::size_of::<c_uint>()) as *mut Fingerprint;

    initStats(fpstats);
    HIST_add(
        (*fpstats).pastEvents.events.as_mut_ptr(),
        blockStart,
        SEGMENT_SIZE,
    );
    HIST_add(
        (*fpstats).newEvents.events.as_mut_ptr(),
        (blockStart as *const c_char).add(blockSize - SEGMENT_SIZE) as *const c_void,
        SEGMENT_SIZE,
    );
    (*fpstats).newEvents.nbEvents = SEGMENT_SIZE;
    (*fpstats).pastEvents.nbEvents = (*fpstats).newEvents.nbEvents;
    if compareFingerprints(
        core::ptr::addr_of!((*fpstats).pastEvents),
        core::ptr::addr_of!((*fpstats).newEvents),
        0,
        8,
    ) == 0
    {
        return blockSize;
    }

    HIST_add(
        (*middleEvents).events.as_mut_ptr(),
        (blockStart as *const c_char).add(blockSize / 2 - SEGMENT_SIZE / 2) as *const c_void,
        SEGMENT_SIZE,
    );
    (*middleEvents).nbEvents = SEGMENT_SIZE;
    {
        let distFromBegin: U64 =
            fpDistance(core::ptr::addr_of!((*fpstats).pastEvents), middleEvents, 8);
        let distFromEnd: U64 =
            fpDistance(core::ptr::addr_of!((*fpstats).newEvents), middleEvents, 8);
        let minDistance: U64 = (SEGMENT_SIZE * SEGMENT_SIZE / 3) as U64;
        if abs64((distFromBegin as S64).wrapping_sub(distFromEnd as S64)) < minDistance {
            return 64 * (1 << 10);
        }
        if distFromBegin > distFromEnd {
            32 * (1 << 10)
        } else {
            96 * (1 << 10)
        }
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
    /* level >= 1 */
    ZSTD_splitBlock_byChunks(blockStart, blockSize, level - 1, workspace, wkspSize)
}
