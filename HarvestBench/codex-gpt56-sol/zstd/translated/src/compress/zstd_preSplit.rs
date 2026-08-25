use ::libc;
extern "C" {
    fn HIST_add(count: *mut ::core::ffi::c_uint, src: *const ::core::ffi::c_void, srcSize: size_t);
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type __int64_t = i64;
pub type __uint64_t = u64;
pub type int64_t = __int64_t;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type BYTE = uint8_t;
pub type U16 = uint16_t;
pub type U32 = uint32_t;
pub type U64 = uint64_t;
pub type S64 = int64_t;
pub type unalign16 = U16;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Fingerprint {
    pub events: [::core::ffi::c_uint; 1024],
    pub nbEvents: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FPStats {
    pub pastEvents: Fingerprint,
    pub newEvents: Fingerprint,
}
pub type RecordEvents_f =
    Option<unsafe extern "C" fn(*mut Fingerprint, *const ::core::ffi::c_void, size_t) -> ()>;
#[inline]
unsafe extern "C" fn MEM_read16(mut ptr: *const ::core::ffi::c_void) -> U16 {
    return *(ptr as *const unalign16);
}
pub const THRESHOLD_PENALTY_RATE: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const THRESHOLD_BASE: ::core::ffi::c_int = THRESHOLD_PENALTY_RATE - 2 as ::core::ffi::c_int;
pub const THRESHOLD_PENALTY: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const HASHLENGTH: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const HASHLOG_MAX: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const HASHTABLESIZE: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << HASHLOG_MAX;
pub const KNUTH: ::core::ffi::c_uint = 0x9e3779b9 as ::core::ffi::c_uint;
#[inline(always)]
unsafe extern "C" fn hash2(
    mut p: *const ::core::ffi::c_void,
    mut hashLog: ::core::ffi::c_uint,
) -> ::core::ffi::c_uint {
    if hashLog == 8 as ::core::ffi::c_uint {
        return *(p as *const BYTE).offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_uint;
    }
    return (MEM_read16(p) as ::core::ffi::c_uint).wrapping_mul(KNUTH)
        >> (32 as ::core::ffi::c_uint).wrapping_sub(hashLog);
}
unsafe extern "C" fn initStats(mut fpstats: *mut FPStats) {
    ::libc::memset(
        fpstats as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<FPStats>() as ::libc::size_t,
    );
}
#[inline(always)]
unsafe extern "C" fn addEvents_generic(
    mut fp: *mut Fingerprint,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut samplingRate: size_t,
    mut hashLog: ::core::ffi::c_uint,
) {
    let mut p: *const ::core::ffi::c_char = src as *const ::core::ffi::c_char;
    let mut limit: size_t = srcSize
        .wrapping_sub(HASHLENGTH as size_t)
        .wrapping_add(1 as size_t);
    let mut n: size_t = 0;
    n = 0 as size_t;
    while n < limit {
        (*fp).events[hash2(p.offset(n as isize) as *const ::core::ffi::c_void, hashLog) as usize] =
            (*fp).events
                [hash2(p.offset(n as isize) as *const ::core::ffi::c_void, hashLog) as usize]
                .wrapping_add(1);
        n = (n as ::core::ffi::c_ulong).wrapping_add(samplingRate as ::core::ffi::c_ulong) as size_t
            as size_t;
    }
    (*fp).nbEvents = ((*fp).nbEvents as ::core::ffi::c_ulong)
        .wrapping_add(limit.wrapping_div(samplingRate) as ::core::ffi::c_ulong)
        as size_t as size_t;
}
#[inline(always)]
unsafe extern "C" fn recordFingerprint_generic(
    mut fp: *mut Fingerprint,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut samplingRate: size_t,
    mut hashLog: ::core::ffi::c_uint,
) {
    ::libc::memset(
        fp as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        (::core::mem::size_of::<::core::ffi::c_uint>() as usize)
            .wrapping_mul((1 as ::core::ffi::c_int as usize) << hashLog) as ::libc::size_t,
    );
    (*fp).nbEvents = 0 as size_t;
    addEvents_generic(fp, src, srcSize, samplingRate, hashLog);
}
unsafe extern "C" fn ZSTD_recordFingerprint_1(
    mut fp: *mut Fingerprint,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) {
    recordFingerprint_generic(fp, src, srcSize, 1 as size_t, 10 as ::core::ffi::c_uint);
}
unsafe extern "C" fn ZSTD_recordFingerprint_5(
    mut fp: *mut Fingerprint,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) {
    recordFingerprint_generic(fp, src, srcSize, 5 as size_t, 10 as ::core::ffi::c_uint);
}
unsafe extern "C" fn ZSTD_recordFingerprint_11(
    mut fp: *mut Fingerprint,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) {
    recordFingerprint_generic(fp, src, srcSize, 11 as size_t, 9 as ::core::ffi::c_uint);
}
unsafe extern "C" fn ZSTD_recordFingerprint_43(
    mut fp: *mut Fingerprint,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) {
    recordFingerprint_generic(fp, src, srcSize, 43 as size_t, 8 as ::core::ffi::c_uint);
}
unsafe extern "C" fn abs64(mut s64: S64) -> U64 {
    return (if s64 < 0 as S64 { -s64 } else { s64 }) as U64;
}
unsafe extern "C" fn fpDistance(
    mut fp1: *const Fingerprint,
    mut fp2: *const Fingerprint,
    mut hashLog: ::core::ffi::c_uint,
) -> U64 {
    let mut distance: U64 = 0 as U64;
    let mut n: size_t = 0;
    n = 0 as size_t;
    while n < (1 as ::core::ffi::c_int as size_t) << hashLog {
        distance = (distance as ::core::ffi::c_ulong).wrapping_add(abs64(
            (*fp1).events[n as usize] as S64 * (*fp2).nbEvents as S64
                - (*fp2).events[n as usize] as S64 * (*fp1).nbEvents as S64,
        )
            as ::core::ffi::c_ulong) as U64 as U64;
        n = n.wrapping_add(1);
    }
    return distance;
}
unsafe extern "C" fn compareFingerprints(
    mut ref_0: *const Fingerprint,
    mut newfp: *const Fingerprint,
    mut penalty: ::core::ffi::c_int,
    mut hashLog: ::core::ffi::c_uint,
) -> ::core::ffi::c_int {
    let mut p50: U64 = ((*ref_0).nbEvents as U64).wrapping_mul((*newfp).nbEvents as U64);
    let mut deviation: U64 = fpDistance(ref_0, newfp, hashLog);
    let mut threshold: U64 = p50
        .wrapping_mul((THRESHOLD_BASE + penalty) as U64)
        .wrapping_div(THRESHOLD_PENALTY_RATE as U64);
    return (deviation >= threshold) as ::core::ffi::c_int;
}
unsafe extern "C" fn mergeEvents(mut acc: *mut Fingerprint, mut newfp: *const Fingerprint) {
    let mut n: size_t = 0;
    n = 0 as size_t;
    while n < HASHTABLESIZE as size_t {
        (*acc).events[n as usize] =
            (*acc).events[n as usize].wrapping_add((*newfp).events[n as usize]);
        n = n.wrapping_add(1);
    }
    (*acc).nbEvents = ((*acc).nbEvents as ::core::ffi::c_ulong)
        .wrapping_add((*newfp).nbEvents as ::core::ffi::c_ulong) as size_t
        as size_t;
}
unsafe extern "C" fn flushEvents(mut fpstats: *mut FPStats) {
    let mut n: size_t = 0;
    n = 0 as size_t;
    while n < HASHTABLESIZE as size_t {
        (*fpstats).pastEvents.events[n as usize] = (*fpstats).newEvents.events[n as usize];
        n = n.wrapping_add(1);
    }
    (*fpstats).pastEvents.nbEvents = (*fpstats).newEvents.nbEvents;
    ::libc::memset(
        &raw mut (*fpstats).newEvents as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<Fingerprint>() as ::libc::size_t,
    );
}
unsafe extern "C" fn removeEvents(mut acc: *mut Fingerprint, mut slice: *const Fingerprint) {
    let mut n: size_t = 0;
    n = 0 as size_t;
    while n < HASHTABLESIZE as size_t {
        (*acc).events[n as usize] =
            (*acc).events[n as usize].wrapping_sub((*slice).events[n as usize]);
        n = n.wrapping_add(1);
    }
    (*acc).nbEvents = ((*acc).nbEvents as ::core::ffi::c_ulong)
        .wrapping_sub((*slice).nbEvents as ::core::ffi::c_ulong) as size_t
        as size_t;
}
pub const CHUNKSIZE: ::core::ffi::c_int = (8 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int;
unsafe extern "C" fn ZSTD_splitBlock_byChunks(
    mut blockStart: *const ::core::ffi::c_void,
    mut blockSize: size_t,
    mut level: ::core::ffi::c_int,
    mut workspace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
) -> size_t {
    static mut records_fs: [RecordEvents_f; 4] = unsafe {
        [
            Some(
                ZSTD_recordFingerprint_43
                    as unsafe extern "C" fn(
                        *mut Fingerprint,
                        *const ::core::ffi::c_void,
                        size_t,
                    ) -> (),
            ),
            Some(
                ZSTD_recordFingerprint_11
                    as unsafe extern "C" fn(
                        *mut Fingerprint,
                        *const ::core::ffi::c_void,
                        size_t,
                    ) -> (),
            ),
            Some(
                ZSTD_recordFingerprint_5
                    as unsafe extern "C" fn(
                        *mut Fingerprint,
                        *const ::core::ffi::c_void,
                        size_t,
                    ) -> (),
            ),
            Some(
                ZSTD_recordFingerprint_1
                    as unsafe extern "C" fn(
                        *mut Fingerprint,
                        *const ::core::ffi::c_void,
                        size_t,
                    ) -> (),
            ),
        ]
    };
    static mut hashParams: [::core::ffi::c_uint; 4] = [
        8 as ::core::ffi::c_int as ::core::ffi::c_uint,
        9 as ::core::ffi::c_int as ::core::ffi::c_uint,
        10 as ::core::ffi::c_int as ::core::ffi::c_uint,
        10 as ::core::ffi::c_int as ::core::ffi::c_uint,
    ];
    let record_f: RecordEvents_f = records_fs[level as usize];
    let fpstats: *mut FPStats = workspace as *mut FPStats;
    let mut p: *const ::core::ffi::c_char = blockStart as *const ::core::ffi::c_char;
    let mut penalty: ::core::ffi::c_int = THRESHOLD_PENALTY;
    let mut pos: size_t = 0 as size_t;
    initStats(fpstats);
    record_f.expect("non-null function pointer")(
        &raw mut (*fpstats).pastEvents,
        p as *const ::core::ffi::c_void,
        CHUNKSIZE as size_t,
    );
    pos = CHUNKSIZE as size_t;
    while pos <= blockSize.wrapping_sub(CHUNKSIZE as size_t) {
        record_f.expect("non-null function pointer")(
            &raw mut (*fpstats).newEvents,
            p.offset(pos as isize) as *const ::core::ffi::c_void,
            CHUNKSIZE as size_t,
        );
        if compareFingerprints(
            &raw mut (*fpstats).pastEvents,
            &raw mut (*fpstats).newEvents,
            penalty,
            hashParams[level as usize],
        ) != 0
        {
            return pos;
        } else {
            mergeEvents(
                &raw mut (*fpstats).pastEvents,
                &raw mut (*fpstats).newEvents,
            );
            if penalty > 0 as ::core::ffi::c_int {
                penalty -= 1;
            }
        }
        pos = (pos as ::core::ffi::c_ulong).wrapping_add(CHUNKSIZE as ::core::ffi::c_ulong)
            as size_t as size_t;
    }
    return blockSize;
}
unsafe extern "C" fn ZSTD_splitBlock_fromBorders(
    mut blockStart: *const ::core::ffi::c_void,
    mut blockSize: size_t,
    mut workspace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
) -> size_t {
    let fpstats: *mut FPStats = workspace as *mut FPStats;
    let mut middleEvents: *mut Fingerprint = (workspace as *mut ::core::ffi::c_char).offset(
        (512 as usize).wrapping_mul(::core::mem::size_of::<::core::ffi::c_uint>() as usize)
            as isize,
    ) as *mut ::core::ffi::c_void as *mut Fingerprint;
    initStats(fpstats);
    HIST_add(
        &raw mut (*fpstats).pastEvents.events as *mut ::core::ffi::c_uint,
        blockStart,
        SEGMENT_SIZE as size_t,
    );
    HIST_add(
        &raw mut (*fpstats).newEvents.events as *mut ::core::ffi::c_uint,
        (blockStart as *const ::core::ffi::c_char)
            .offset(blockSize as isize)
            .offset(-(SEGMENT_SIZE as isize)) as *const ::core::ffi::c_void,
        SEGMENT_SIZE as size_t,
    );
    (*fpstats).newEvents.nbEvents = SEGMENT_SIZE as size_t;
    (*fpstats).pastEvents.nbEvents = (*fpstats).newEvents.nbEvents;
    if compareFingerprints(
        &raw mut (*fpstats).pastEvents,
        &raw mut (*fpstats).newEvents,
        0 as ::core::ffi::c_int,
        8 as ::core::ffi::c_uint,
    ) == 0
    {
        return blockSize;
    }
    HIST_add(
        &raw mut (*middleEvents).events as *mut ::core::ffi::c_uint,
        (blockStart as *const ::core::ffi::c_char)
            .offset(blockSize.wrapping_div(2 as size_t) as isize)
            .offset(-((SEGMENT_SIZE / 2 as ::core::ffi::c_int) as isize))
            as *const ::core::ffi::c_void,
        SEGMENT_SIZE as size_t,
    );
    (*middleEvents).nbEvents = SEGMENT_SIZE as size_t;
    let distFromBegin: U64 = fpDistance(
        &raw mut (*fpstats).pastEvents,
        middleEvents,
        8 as ::core::ffi::c_uint,
    ) as U64;
    let distFromEnd: U64 = fpDistance(
        &raw mut (*fpstats).newEvents,
        middleEvents,
        8 as ::core::ffi::c_uint,
    ) as U64;
    let minDistance: U64 = (SEGMENT_SIZE * SEGMENT_SIZE / 3 as ::core::ffi::c_int) as U64;
    if abs64(distFromBegin as S64 - distFromEnd as S64) < minDistance {
        return (64 as ::core::ffi::c_int * ((1 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int))
            as size_t;
    }
    return (if distFromBegin > distFromEnd {
        32 as ::core::ffi::c_int * ((1 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int)
    } else {
        96 as ::core::ffi::c_int * ((1 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int)
    }) as size_t;
}
pub const SEGMENT_SIZE: ::core::ffi::c_int = 512 as ::core::ffi::c_int;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_splitBlock(
    mut blockStart: *const ::core::ffi::c_void,
    mut blockSize: size_t,
    mut level: ::core::ffi::c_int,
    mut workspace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
) -> size_t {
    if level == 0 as ::core::ffi::c_int {
        return ZSTD_splitBlock_fromBorders(blockStart, blockSize, workspace, wkspSize);
    }
    return ZSTD_splitBlock_byChunks(
        blockStart,
        blockSize,
        level - 1 as ::core::ffi::c_int,
        workspace,
        wkspSize,
    );
}
