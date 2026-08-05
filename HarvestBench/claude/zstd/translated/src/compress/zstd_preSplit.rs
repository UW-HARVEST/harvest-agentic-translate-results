/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 * All rights reserved.
 *
 * This source code is licensed under both the BSD-style license (found in the
 * LICENSE file in the root directory of this source tree) and the GPLv2 (found
 * in the COPYING file in the root directory of this source tree).
 * You may select, at your option, one of the above-listed licenses.
 */

use core::ffi::{c_int, c_uint, c_void};

use crate::common::mem::mem_read16;
use crate::compress::hist::HIST_add;

const BLOCKSIZE_MIN: usize = 3500;
const THRESHOLD_PENALTY_RATE: u64 = 16;
const THRESHOLD_BASE: u64 = THRESHOLD_PENALTY_RATE - 2;
const THRESHOLD_PENALTY: i32 = 3;

const HASHLENGTH: usize = 2;
const HASHLOG_MAX: c_uint = 10;
const HASHTABLESIZE: usize = 1 << HASHLOG_MAX;
const HASHMASK: usize = HASHTABLESIZE - 1;
const KNUTH: u32 = 0x9e3779b9;

/* for hashLog > 8, hash 2 bytes.
 * for hashLog == 8, just take the byte, no hashing.
 * The speed of this method relies on compile-time constant propagation */
#[inline(always)]
unsafe fn hash2(p: *const c_void, hashLog: c_uint) -> c_uint {
    debug_assert!(hashLog >= 8);
    if hashLog == 8 {
        return *(p as *const u8) as u32;
    }
    debug_assert!(hashLog <= HASHLOG_MAX);
    ((mem_read16(p) as u32).wrapping_mul(KNUTH) >> (32 - hashLog)) as c_uint
}

#[repr(C)]
struct Fingerprint {
    events: [c_uint; HASHTABLESIZE],
    nbEvents: usize,
}

#[repr(C)]
struct FPStats {
    pastEvents: Fingerprint,
    newEvents: Fingerprint,
}

unsafe fn initStats(fpstats: *mut FPStats) {
    core::ptr::write_bytes(fpstats as *mut u8, 0, core::mem::size_of::<FPStats>());
}

#[inline(always)]
unsafe fn addEvents_generic(
    fp: *mut Fingerprint,
    src: *const c_void,
    srcSize: usize,
    samplingRate: usize,
    hashLog: c_uint,
) {
    let p = src as *const i8;
    let limit = srcSize - HASHLENGTH + 1;
    let mut n: usize;
    debug_assert!(srcSize >= HASHLENGTH);
    n = 0;
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
    core::ptr::write_bytes(
        fp as *mut u8,
        0,
        core::mem::size_of::<c_uint>() * (1usize << hashLog),
    );
    (*fp).nbEvents = 0;
    addEvents_generic(fp, src, srcSize, samplingRate, hashLog);
}

type RecordEvents_f = unsafe fn(fp: *mut Fingerprint, src: *const c_void, srcSize: usize);

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

fn abs64(s64: i64) -> u64 {
    (if s64 < 0 { -s64 } else { s64 }) as u64
}

unsafe fn fpDistance(fp1: *const Fingerprint, fp2: *const Fingerprint, hashLog: c_uint) -> u64 {
    let mut distance: u64 = 0;
    let mut n: usize;
    debug_assert!(hashLog <= HASHLOG_MAX);
    n = 0;
    while n < (1usize << hashLog) {
        distance = distance.wrapping_add(abs64(
            ((*fp1).events[n] as i64)
                .wrapping_mul((*fp2).nbEvents as i64)
                .wrapping_sub(((*fp2).events[n] as i64).wrapping_mul((*fp1).nbEvents as i64)),
        ));
        n += 1;
    }
    distance
}

/* Compare newEvents with pastEvents
 * return 1 when considered "too different"
 */
unsafe fn compareFingerprints(
    r#ref: *const Fingerprint,
    newfp: *const Fingerprint,
    penalty: c_int,
    hashLog: c_uint,
) -> c_int {
    debug_assert!((*r#ref).nbEvents > 0);
    debug_assert!((*newfp).nbEvents > 0);
    {
        let p50: u64 = ((*r#ref).nbEvents as u64).wrapping_mul((*newfp).nbEvents as u64);
        let deviation: u64 = fpDistance(r#ref, newfp, hashLog);
        let threshold: u64 = p50
            .wrapping_mul((THRESHOLD_BASE as i64 + penalty as i64) as u64)
            / THRESHOLD_PENALTY_RATE;
        (deviation >= threshold) as c_int
    }
}

unsafe fn mergeEvents(acc: *mut Fingerprint, newfp: *const Fingerprint) {
    let mut n: usize;
    n = 0;
    while n < HASHTABLESIZE {
        (*acc).events[n] = (*acc).events[n].wrapping_add((*newfp).events[n]);
        n += 1;
    }
    (*acc).nbEvents += (*newfp).nbEvents;
}

unsafe fn flushEvents(fpstats: *mut FPStats) {
    let mut n: usize;
    n = 0;
    while n < HASHTABLESIZE {
        (*fpstats).pastEvents.events[n] = (*fpstats).newEvents.events[n];
        n += 1;
    }
    (*fpstats).pastEvents.nbEvents = (*fpstats).newEvents.nbEvents;
    core::ptr::write_bytes(
        &mut (*fpstats).newEvents as *mut Fingerprint as *mut u8,
        0,
        core::mem::size_of::<Fingerprint>(),
    );
}

unsafe fn removeEvents(acc: *mut Fingerprint, slice: *const Fingerprint) {
    let mut n: usize;
    n = 0;
    while n < HASHTABLESIZE {
        debug_assert!((*acc).events[n] >= (*slice).events[n]);
        (*acc).events[n] = (*acc).events[n].wrapping_sub((*slice).events[n]);
        n += 1;
    }
    (*acc).nbEvents -= (*slice).nbEvents;
}

const CHUNKSIZE: usize = 8 << 10;
unsafe fn ZSTD_splitBlock_byChunks(
    blockStart: *const c_void,
    blockSize: usize,
    level: c_int,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    static records_fs: [RecordEvents_f; 4] = [
        ZSTD_recordFingerprint_43,
        ZSTD_recordFingerprint_11,
        ZSTD_recordFingerprint_5,
        ZSTD_recordFingerprint_1,
    ];
    static hashParams: [c_uint; 4] = [8, 9, 10, 10];
    debug_assert!(0 <= level && level <= 3);
    let record_f: RecordEvents_f = records_fs[level as usize];
    let fpstats = workspace as *mut FPStats;
    let p = blockStart as *const i8;
    let mut penalty: c_int = THRESHOLD_PENALTY;
    let mut pos: usize = 0;
    debug_assert!(blockSize == (128 << 10));
    debug_assert!(!workspace.is_null());
    debug_assert!(workspace as usize % core::mem::align_of::<FPStats>() == 0);
    let _ = wkspSize;
    debug_assert!(wkspSize >= core::mem::size_of::<FPStats>());

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
    debug_assert!(pos == blockSize);
    return blockSize;
}

/* ZSTD_splitBlock_fromBorders(): very fast strategy :
 * compare fingerprint from beginning and end of the block,
 * derive from their difference if it's preferable to split in the middle,
 * repeat the process a second time, for finer grained decision.
 * 3 times did not brought improvements, so I stopped at 2.
 * Benefits are good enough for a cheap heuristic.
 * More accurate splitting saves more, but speed impact is also more perceptible.
 * For better accuracy, use more elaborate variant *_byChunks.
 */
const SEGMENT_SIZE: usize = 512;
unsafe fn ZSTD_splitBlock_fromBorders(
    blockStart: *const c_void,
    blockSize: usize,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let fpstats = workspace as *mut FPStats;
    let middleEvents =
        (workspace as *mut i8).add(512 * core::mem::size_of::<c_uint>()) as *mut Fingerprint;
    debug_assert!(blockSize == (128 << 10));
    debug_assert!(!workspace.is_null());
    debug_assert!(workspace as usize % core::mem::align_of::<FPStats>() == 0);
    let _ = wkspSize;
    debug_assert!(wkspSize >= core::mem::size_of::<FPStats>());

    initStats(fpstats);
    HIST_add(
        (*fpstats).pastEvents.events.as_mut_ptr(),
        blockStart,
        SEGMENT_SIZE,
    );
    HIST_add(
        (*fpstats).newEvents.events.as_mut_ptr(),
        (blockStart as *const i8).add(blockSize - SEGMENT_SIZE) as *const c_void,
        SEGMENT_SIZE,
    );
    (*fpstats).newEvents.nbEvents = SEGMENT_SIZE;
    (*fpstats).pastEvents.nbEvents = (*fpstats).newEvents.nbEvents;
    if compareFingerprints(&(*fpstats).pastEvents, &(*fpstats).newEvents, 0, 8) == 0 {
        return blockSize;
    }

    HIST_add(
        (*middleEvents).events.as_mut_ptr(),
        (blockStart as *const i8).add(blockSize / 2 - SEGMENT_SIZE / 2) as *const c_void,
        SEGMENT_SIZE,
    );
    (*middleEvents).nbEvents = SEGMENT_SIZE;
    {
        let distFromBegin: u64 = fpDistance(&(*fpstats).pastEvents, middleEvents, 8);
        let distFromEnd: u64 = fpDistance(&(*fpstats).newEvents, middleEvents, 8);
        let minDistance: u64 = (SEGMENT_SIZE * SEGMENT_SIZE / 3) as u64;
        if abs64((distFromBegin as i64).wrapping_sub(distFromEnd as i64)) < minDistance {
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
    debug_assert!(0 <= level && level <= 4);
    if level == 0 {
        return ZSTD_splitBlock_fromBorders(blockStart, blockSize, workspace, wkspSize);
    }
    /* level >= 1*/
    ZSTD_splitBlock_byChunks(blockStart, blockSize, level - 1, workspace, wkspSize)
}
