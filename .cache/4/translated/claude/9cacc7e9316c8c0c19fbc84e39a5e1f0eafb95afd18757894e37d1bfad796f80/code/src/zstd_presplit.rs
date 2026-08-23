//! Translation of compress/zstd_preSplit.c (+ compress/zstd_preSplit.h)
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

use core::ptr::{addr_of, addr_of_mut};

use crate::hist::HIST_add;
use crate::mem::*;

/* #define ZSTD_SLIPBLOCK_WORKSPACESIZE 8208 (zstd_preSplit.h) */
pub use crate::zstd_compress_internal::ZSTD_SLIPBLOCK_WORKSPACESIZE;

pub const BLOCKSIZE_MIN: usize = 3500;
pub const THRESHOLD_PENALTY_RATE: core::ffi::c_int = 16;
pub const THRESHOLD_BASE: core::ffi::c_int = THRESHOLD_PENALTY_RATE - 2;
pub const THRESHOLD_PENALTY: core::ffi::c_int = 3;

pub const HASHLENGTH: usize = 2;
pub const HASHLOG_MAX: core::ffi::c_uint = 10;
pub const HASHTABLESIZE: usize = 1usize << HASHLOG_MAX;
pub const HASHMASK: usize = HASHTABLESIZE - 1;
pub const KNUTH: U32 = 0x9e3779b9;

/* for hashLog > 8, hash 2 bytes.
 * for hashLog == 8, just take the byte, no hashing.
 * The speed of this method relies on compile-time constant propagation */
#[inline(always)]
pub unsafe fn hash2(p: *const core::ffi::c_void, hashLog: core::ffi::c_uint) -> core::ffi::c_uint {
    if hashLog == 8 {
        return *(p as *const BYTE) as U32;
    }
    (MEM_read16(p as *const BYTE) as U32).wrapping_mul(KNUTH) >> (32 - hashLog)
}

#[repr(C)]
pub struct Fingerprint {
    pub events: [core::ffi::c_uint; HASHTABLESIZE],
    pub nbEvents: usize,
}

#[repr(C)]
pub struct FPStats {
    pub pastEvents: Fingerprint,
    pub newEvents: Fingerprint,
}

pub unsafe fn initStats(fpstats: *mut FPStats) {
    ZSTD_memset(fpstats as *mut u8, 0, core::mem::size_of::<FPStats>());
}

#[inline(always)]
pub unsafe fn addEvents_generic(
    fp: *mut Fingerprint,
    src: *const core::ffi::c_void,
    srcSize: usize,
    samplingRate: usize,
    hashLog: core::ffi::c_uint,
) {
    let p: *const core::ffi::c_char = src as *const core::ffi::c_char;
    let limit: usize = srcSize.wrapping_sub(HASHLENGTH).wrapping_add(1);
    let mut n: usize;
    let events: *mut core::ffi::c_uint = addr_of_mut!((*fp).events) as *mut core::ffi::c_uint;
    n = 0;
    while n < limit {
        let h: usize = hash2(p.wrapping_add(n) as *const core::ffi::c_void, hashLog) as usize;
        *events.add(h) = (*events.add(h)).wrapping_add(1);
        n = n.wrapping_add(samplingRate);
    }
    (*fp).nbEvents = (*fp).nbEvents.wrapping_add(limit / samplingRate);
}

#[inline(always)]
pub unsafe fn recordFingerprint_generic(
    fp: *mut Fingerprint,
    src: *const core::ffi::c_void,
    srcSize: usize,
    samplingRate: usize,
    hashLog: core::ffi::c_uint,
) {
    ZSTD_memset(
        fp as *mut u8,
        0,
        core::mem::size_of::<core::ffi::c_uint>() * (1usize << hashLog),
    );
    (*fp).nbEvents = 0;
    addEvents_generic(fp, src, srcSize, samplingRate, hashLog);
}

pub type RecordEvents_f = unsafe fn(*mut Fingerprint, *const core::ffi::c_void, usize);

/* ZSTD_GEN_RECORD_FINGERPRINT(1, 10) */
pub unsafe fn ZSTD_recordFingerprint_1(
    fp: *mut Fingerprint,
    src: *const core::ffi::c_void,
    srcSize: usize,
) {
    recordFingerprint_generic(fp, src, srcSize, 1, 10);
}

/* ZSTD_GEN_RECORD_FINGERPRINT(5, 10) */
pub unsafe fn ZSTD_recordFingerprint_5(
    fp: *mut Fingerprint,
    src: *const core::ffi::c_void,
    srcSize: usize,
) {
    recordFingerprint_generic(fp, src, srcSize, 5, 10);
}

/* ZSTD_GEN_RECORD_FINGERPRINT(11, 9) */
pub unsafe fn ZSTD_recordFingerprint_11(
    fp: *mut Fingerprint,
    src: *const core::ffi::c_void,
    srcSize: usize,
) {
    recordFingerprint_generic(fp, src, srcSize, 11, 9);
}

/* ZSTD_GEN_RECORD_FINGERPRINT(43, 8) */
pub unsafe fn ZSTD_recordFingerprint_43(
    fp: *mut Fingerprint,
    src: *const core::ffi::c_void,
    srcSize: usize,
) {
    recordFingerprint_generic(fp, src, srcSize, 43, 8);
}

pub fn abs64(s64: S64) -> U64 {
    (if s64 < 0 { s64.wrapping_neg() } else { s64 }) as U64
}

pub unsafe fn fpDistance(
    fp1: *const Fingerprint,
    fp2: *const Fingerprint,
    hashLog: core::ffi::c_uint,
) -> U64 {
    let mut distance: U64 = 0;
    let mut n: usize;
    let ev1: *const core::ffi::c_uint = addr_of!((*fp1).events) as *const core::ffi::c_uint;
    let ev2: *const core::ffi::c_uint = addr_of!((*fp2).events) as *const core::ffi::c_uint;
    n = 0;
    while n < (1usize << hashLog) {
        distance = distance.wrapping_add(abs64(
            (*ev1.add(n) as S64)
                .wrapping_mul((*fp2).nbEvents as S64)
                .wrapping_sub((*ev2.add(n) as S64).wrapping_mul((*fp1).nbEvents as S64)),
        ));
        n = n.wrapping_add(1);
    }
    distance
}

/* Compare newEvents with pastEvents
 * return 1 when considered "too different"
 */
pub unsafe fn compareFingerprints(
    r#ref: *const Fingerprint,
    newfp: *const Fingerprint,
    penalty: core::ffi::c_int,
    hashLog: core::ffi::c_uint,
) -> core::ffi::c_int {
    {
        let p50: U64 = ((*r#ref).nbEvents as U64).wrapping_mul((*newfp).nbEvents as U64);
        let deviation: U64 = fpDistance(r#ref, newfp, hashLog);
        let threshold: U64 = p50
            .wrapping_mul(THRESHOLD_BASE.wrapping_add(penalty) as U64)
            / (THRESHOLD_PENALTY_RATE as U64);
        return (deviation >= threshold) as core::ffi::c_int;
    }
}

pub unsafe fn mergeEvents(acc: *mut Fingerprint, newfp: *const Fingerprint) {
    let mut n: usize;
    let accEv: *mut core::ffi::c_uint = addr_of_mut!((*acc).events) as *mut core::ffi::c_uint;
    let newEv: *const core::ffi::c_uint = addr_of!((*newfp).events) as *const core::ffi::c_uint;
    n = 0;
    while n < HASHTABLESIZE {
        *accEv.add(n) = (*accEv.add(n)).wrapping_add(*newEv.add(n));
        n = n.wrapping_add(1);
    }
    (*acc).nbEvents = (*acc).nbEvents.wrapping_add((*newfp).nbEvents);
}

pub unsafe fn flushEvents(fpstats: *mut FPStats) {
    let mut n: usize;
    let pastEv: *mut core::ffi::c_uint =
        addr_of_mut!((*fpstats).pastEvents.events) as *mut core::ffi::c_uint;
    let newEv: *const core::ffi::c_uint =
        addr_of!((*fpstats).newEvents.events) as *const core::ffi::c_uint;
    n = 0;
    while n < HASHTABLESIZE {
        *pastEv.add(n) = *newEv.add(n);
        n = n.wrapping_add(1);
    }
    (*fpstats).pastEvents.nbEvents = (*fpstats).newEvents.nbEvents;
    ZSTD_memset(
        addr_of_mut!((*fpstats).newEvents) as *mut u8,
        0,
        core::mem::size_of::<Fingerprint>(),
    );
}

pub unsafe fn removeEvents(acc: *mut Fingerprint, slice: *const Fingerprint) {
    let mut n: usize;
    let accEv: *mut core::ffi::c_uint = addr_of_mut!((*acc).events) as *mut core::ffi::c_uint;
    let sliceEv: *const core::ffi::c_uint = addr_of!((*slice).events) as *const core::ffi::c_uint;
    n = 0;
    while n < HASHTABLESIZE {
        *accEv.add(n) = (*accEv.add(n)).wrapping_sub(*sliceEv.add(n));
        n = n.wrapping_add(1);
    }
    (*acc).nbEvents = (*acc).nbEvents.wrapping_sub((*slice).nbEvents);
}

pub const CHUNKSIZE: usize = 8 << 10;

/* `static const RecordEvents_f records_fs[]` local to ZSTD_splitBlock_byChunks */
pub static records_fs: [RecordEvents_f; 4] = [
    ZSTD_recordFingerprint_43,
    ZSTD_recordFingerprint_11,
    ZSTD_recordFingerprint_5,
    ZSTD_recordFingerprint_1,
];
/* `static const unsigned hashParams[]` local to ZSTD_splitBlock_byChunks */
pub static hashParams: [core::ffi::c_uint; 4] = [8, 9, 10, 10];

pub unsafe fn ZSTD_splitBlock_byChunks(
    blockStart: *const core::ffi::c_void,
    blockSize: usize,
    level: core::ffi::c_int,
    workspace: *mut core::ffi::c_void,
    wkspSize: usize,
) -> usize {
    let record_f: RecordEvents_f = records_fs[level as usize];
    let fpstats: *mut FPStats = workspace as *mut FPStats;
    let p: *const core::ffi::c_char = blockStart as *const core::ffi::c_char;
    let mut penalty: core::ffi::c_int = THRESHOLD_PENALTY;
    let mut pos: usize = 0;

    initStats(fpstats);
    record_f(
        addr_of_mut!((*fpstats).pastEvents),
        p as *const core::ffi::c_void,
        CHUNKSIZE,
    );
    pos = CHUNKSIZE;
    while pos <= blockSize.wrapping_sub(CHUNKSIZE) {
        record_f(
            addr_of_mut!((*fpstats).newEvents),
            p.wrapping_add(pos) as *const core::ffi::c_void,
            CHUNKSIZE,
        );
        if compareFingerprints(
            addr_of!((*fpstats).pastEvents),
            addr_of!((*fpstats).newEvents),
            penalty,
            hashParams[level as usize],
        ) != 0
        {
            return pos;
        } else {
            mergeEvents(
                addr_of_mut!((*fpstats).pastEvents),
                addr_of!((*fpstats).newEvents),
            );
            if penalty > 0 {
                penalty -= 1;
            }
        }
        pos = pos.wrapping_add(CHUNKSIZE);
    }
    blockSize
}

pub const SEGMENT_SIZE: usize = 512;

/* ZSTD_splitBlock_fromBorders(): very fast strategy :
 * compare fingerprint from beginning and end of the block,
 * derive from their difference if it's preferable to split in the middle,
 * repeat the process a second time, for finer grained decision.
 * 3 times did not brought improvements, so I stopped at 2.
 * Benefits are good enough for a cheap heuristic.
 * More accurate splitting saves more, but speed impact is also more perceptible.
 * For better accuracy, use more elaborate variant *_byChunks.
 */
pub unsafe fn ZSTD_splitBlock_fromBorders(
    blockStart: *const core::ffi::c_void,
    blockSize: usize,
    workspace: *mut core::ffi::c_void,
    wkspSize: usize,
) -> usize {
    let fpstats: *mut FPStats = workspace as *mut FPStats;
    let middleEvents: *mut Fingerprint = (workspace as *mut core::ffi::c_char)
        .wrapping_add(512 * core::mem::size_of::<core::ffi::c_uint>())
        as *mut core::ffi::c_void as *mut Fingerprint;

    initStats(fpstats);
    HIST_add(
        addr_of_mut!((*fpstats).pastEvents.events) as *mut core::ffi::c_uint,
        blockStart,
        SEGMENT_SIZE,
    );
    HIST_add(
        addr_of_mut!((*fpstats).newEvents.events) as *mut core::ffi::c_uint,
        (blockStart as *const core::ffi::c_char)
            .wrapping_add(blockSize.wrapping_sub(SEGMENT_SIZE)) as *const core::ffi::c_void,
        SEGMENT_SIZE,
    );
    (*fpstats).newEvents.nbEvents = SEGMENT_SIZE;
    (*fpstats).pastEvents.nbEvents = (*fpstats).newEvents.nbEvents;
    if compareFingerprints(
        addr_of!((*fpstats).pastEvents),
        addr_of!((*fpstats).newEvents),
        0,
        8,
    ) == 0
    {
        return blockSize;
    }

    HIST_add(
        addr_of_mut!((*middleEvents).events) as *mut core::ffi::c_uint,
        (blockStart as *const core::ffi::c_char)
            .wrapping_add(blockSize / 2)
            .wrapping_sub(SEGMENT_SIZE / 2) as *const core::ffi::c_void,
        SEGMENT_SIZE,
    );
    (*middleEvents).nbEvents = SEGMENT_SIZE;
    {
        let distFromBegin: U64 = fpDistance(addr_of!((*fpstats).pastEvents), middleEvents, 8);
        let distFromEnd: U64 = fpDistance(addr_of!((*fpstats).newEvents), middleEvents, 8);
        let minDistance: U64 = (SEGMENT_SIZE * SEGMENT_SIZE / 3) as U64;
        if abs64((distFromBegin as S64).wrapping_sub(distFromEnd as S64)) < minDistance {
            return 64 * 1024;
        }
        return if distFromBegin > distFromEnd {
            32 * 1024
        } else {
            96 * 1024
        };
    }
}

/// `size_t ZSTD_splitBlock(const void* blockStart, size_t blockSize,
///                     int level, void* workspace, size_t wkspSize)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_splitBlock(
    blockStart: *const core::ffi::c_void,
    blockSize: usize,
    level: core::ffi::c_int,
    workspace: *mut core::ffi::c_void,
    wkspSize: usize,
) -> usize {
    if level == 0 {
        return ZSTD_splitBlock_fromBorders(blockStart, blockSize, workspace, wkspSize);
    }
    /* level >= 1*/
    ZSTD_splitBlock_byChunks(blockStart, blockSize, level - 1, workspace, wkspSize)
}
