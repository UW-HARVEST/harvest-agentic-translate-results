extern "C" {
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    pub type POOL_ctx_s;
    static mut stderr: *mut FILE;
    fn fflush(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn clock() -> clock_t;
    fn POOL_create(numThreads: size_t, queueSize: size_t) -> *mut POOL_ctx;
    fn POOL_free(ctx: *mut POOL_ctx);
    fn POOL_add(ctx: *mut POOL_ctx, function: POOL_function, opaque: *mut ::core::ffi::c_void);
    fn ZDICT_finalizeDictionary(
        dstDictBuffer: *mut ::core::ffi::c_void,
        maxDictSize: size_t,
        dictContent: *const ::core::ffi::c_void,
        dictContentSize: size_t,
        samplesBuffer: *const ::core::ffi::c_void,
        samplesSizes: *const size_t,
        nbSamples: ::core::ffi::c_uint,
        parameters: ZDICT_params_t,
    ) -> size_t;
    fn COVER_computeEpochs(
        maxDictSize: U32,
        nbDmers: U32,
        k: U32,
        passes: U32,
    ) -> COVER_epoch_info_t;
    fn COVER_warnOnSmallCorpus(
        maxDictSize: size_t,
        nbDmers: size_t,
        displayLevel: ::core::ffi::c_int,
    );
    fn COVER_sum(samplesSizes: *const size_t, nbSamples: ::core::ffi::c_uint) -> size_t;
    fn COVER_best_init(best: *mut COVER_best_t);
    fn COVER_best_wait(best: *mut COVER_best_t);
    fn COVER_best_destroy(best: *mut COVER_best_t);
    fn COVER_best_start(best: *mut COVER_best_t);
    fn COVER_best_finish(
        best: *mut COVER_best_t,
        parameters: ZDICT_cover_params_t,
        selection: COVER_dictSelection_t,
    );
    fn COVER_dictSelectionIsError(selection: COVER_dictSelection_t) -> ::core::ffi::c_uint;
    fn COVER_dictSelectionError(error: size_t) -> COVER_dictSelection_t;
    fn COVER_dictSelectionFree(selection: COVER_dictSelection_t);
    fn COVER_selectDict(
        customDictContent: *mut BYTE,
        dictBufferCapacity: size_t,
        dictContentSize: size_t,
        samplesBuffer: *const BYTE,
        samplesSizes: *const size_t,
        nbFinalizeSamples: ::core::ffi::c_uint,
        nbCheckSamples: size_t,
        nbSamples: size_t,
        params: ZDICT_cover_params_t,
        offsets: *mut size_t,
        totalCompressedSize: size_t,
    ) -> COVER_dictSelection_t;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
pub type __clock_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: ::core::ffi::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub _codecvt: *mut _IO_codecvt,
    pub _wide_data: *mut _IO_wide_data,
    pub _freeres_list: *mut _IO_FILE,
    pub _freeres_buf: *mut ::core::ffi::c_void,
    pub __pad5: size_t,
    pub _mode: ::core::ffi::c_int,
    pub _unused2: [::core::ffi::c_char; 20],
}
pub type _IO_lock_t = ();
pub type FILE = _IO_FILE;
pub type clock_t = __clock_t;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type BYTE = uint8_t;
pub type U16 = uint16_t;
pub type U32 = uint32_t;
pub type U64 = uint64_t;
pub type unalign64 = U64;
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const ZSTD_error_maxCode: C2RustUnnamed = 120;
pub const ZSTD_error_externalSequences_invalid: C2RustUnnamed = 107;
pub const ZSTD_error_sequenceProducer_failed: C2RustUnnamed = 106;
pub const ZSTD_error_srcBuffer_wrong: C2RustUnnamed = 105;
pub const ZSTD_error_dstBuffer_wrong: C2RustUnnamed = 104;
pub const ZSTD_error_seekableIO: C2RustUnnamed = 102;
pub const ZSTD_error_frameIndex_tooLarge: C2RustUnnamed = 100;
pub const ZSTD_error_noForwardProgress_inputEmpty: C2RustUnnamed = 82;
pub const ZSTD_error_noForwardProgress_destFull: C2RustUnnamed = 80;
pub const ZSTD_error_dstBuffer_null: C2RustUnnamed = 74;
pub const ZSTD_error_srcSize_wrong: C2RustUnnamed = 72;
pub const ZSTD_error_dstSize_tooSmall: C2RustUnnamed = 70;
pub const ZSTD_error_workSpace_tooSmall: C2RustUnnamed = 66;
pub const ZSTD_error_memory_allocation: C2RustUnnamed = 64;
pub const ZSTD_error_init_missing: C2RustUnnamed = 62;
pub const ZSTD_error_stage_wrong: C2RustUnnamed = 60;
pub const ZSTD_error_stabilityCondition_notRespected: C2RustUnnamed = 50;
pub const ZSTD_error_cannotProduce_uncompressedBlock: C2RustUnnamed = 49;
pub const ZSTD_error_maxSymbolValue_tooSmall: C2RustUnnamed = 48;
pub const ZSTD_error_maxSymbolValue_tooLarge: C2RustUnnamed = 46;
pub const ZSTD_error_tableLog_tooLarge: C2RustUnnamed = 44;
pub const ZSTD_error_parameter_outOfBound: C2RustUnnamed = 42;
pub const ZSTD_error_parameter_combination_unsupported: C2RustUnnamed = 41;
pub const ZSTD_error_parameter_unsupported: C2RustUnnamed = 40;
pub const ZSTD_error_dictionaryCreation_failed: C2RustUnnamed = 34;
pub const ZSTD_error_dictionary_wrong: C2RustUnnamed = 32;
pub const ZSTD_error_dictionary_corrupted: C2RustUnnamed = 30;
pub const ZSTD_error_literals_headerWrong: C2RustUnnamed = 24;
pub const ZSTD_error_checksum_wrong: C2RustUnnamed = 22;
pub const ZSTD_error_corruption_detected: C2RustUnnamed = 20;
pub const ZSTD_error_frameParameter_windowTooLarge: C2RustUnnamed = 16;
pub const ZSTD_error_frameParameter_unsupported: C2RustUnnamed = 14;
pub const ZSTD_error_version_unsupported: C2RustUnnamed = 12;
pub const ZSTD_error_prefix_unknown: C2RustUnnamed = 10;
pub const ZSTD_error_GENERIC: C2RustUnnamed = 1;
pub const ZSTD_error_no_error: C2RustUnnamed = 0;
pub type POOL_ctx = POOL_ctx_s;
pub type POOL_function = Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> ()>;
pub type ZSTD_pthread_mutex_t = ::core::ffi::c_int;
pub type ZSTD_pthread_cond_t = ::core::ffi::c_int;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZDICT_params_t {
    pub compressionLevel: ::core::ffi::c_int,
    pub notificationLevel: ::core::ffi::c_uint,
    pub dictID: ::core::ffi::c_uint,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZDICT_cover_params_t {
    pub k: ::core::ffi::c_uint,
    pub d: ::core::ffi::c_uint,
    pub steps: ::core::ffi::c_uint,
    pub nbThreads: ::core::ffi::c_uint,
    pub splitPoint: ::core::ffi::c_double,
    pub shrinkDict: ::core::ffi::c_uint,
    pub shrinkDictMaxRegression: ::core::ffi::c_uint,
    pub zParams: ZDICT_params_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZDICT_fastCover_params_t {
    pub k: ::core::ffi::c_uint,
    pub d: ::core::ffi::c_uint,
    pub f: ::core::ffi::c_uint,
    pub steps: ::core::ffi::c_uint,
    pub nbThreads: ::core::ffi::c_uint,
    pub splitPoint: ::core::ffi::c_double,
    pub accel: ::core::ffi::c_uint,
    pub shrinkDict: ::core::ffi::c_uint,
    pub shrinkDictMaxRegression: ::core::ffi::c_uint,
    pub zParams: ZDICT_params_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FASTCOVER_accel_t {
    pub finalize: ::core::ffi::c_uint,
    pub skip: ::core::ffi::c_uint,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FASTCOVER_ctx_t {
    pub samples: *const BYTE,
    pub offsets: *mut size_t,
    pub samplesSizes: *const size_t,
    pub nbSamples: size_t,
    pub nbTrainSamples: size_t,
    pub nbTestSamples: size_t,
    pub nbDmers: size_t,
    pub freqs: *mut U32,
    pub d: ::core::ffi::c_uint,
    pub f: ::core::ffi::c_uint,
    pub accelParams: FASTCOVER_accel_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct COVER_segment_t {
    pub begin: U32,
    pub end: U32,
    pub score: U32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct COVER_epoch_info_t {
    pub num: U32,
    pub size: U32,
}
pub type COVER_best_t = COVER_best_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct COVER_best_s {
    pub mutex: ZSTD_pthread_mutex_t,
    pub cond: ZSTD_pthread_cond_t,
    pub liveJobs: size_t,
    pub dict: *mut ::core::ffi::c_void,
    pub dictSize: size_t,
    pub parameters: ZDICT_cover_params_t,
    pub compressedSize: size_t,
}
pub type FASTCOVER_tryParameters_data_t = FASTCOVER_tryParameters_data_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FASTCOVER_tryParameters_data_s {
    pub ctx: *const FASTCOVER_ctx_t,
    pub best: *mut COVER_best_t,
    pub dictBufferCapacity: size_t,
    pub parameters: ZDICT_cover_params_t,
}
pub type COVER_dictSelection_t = COVER_dictSelection;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct COVER_dictSelection {
    pub dictContent: *mut BYTE,
    pub dictSize: size_t,
    pub totalCompressedSize: size_t,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const CLOCKS_PER_SEC: __clock_t = 1000000 as ::core::ffi::c_int as __clock_t;
#[inline]
unsafe extern "C" fn MEM_isLittleEndian() -> ::core::ffi::c_uint {
    return 1 as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn MEM_read64(mut ptr: *const ::core::ffi::c_void) -> U64 {
    return *(ptr as *const unalign64);
}
#[inline]
unsafe extern "C" fn MEM_swap64(mut in_0: U64) -> U64 {
    return in_0.swap_bytes();
}
#[inline]
unsafe extern "C" fn MEM_readLE64(mut memPtr: *const ::core::ffi::c_void) -> U64 {
    if MEM_isLittleEndian() != 0 {
        return MEM_read64(memPtr);
    } else {
        return MEM_swap64(MEM_read64(memPtr));
    };
}
static mut prime6bytes: U64 = 227718039650203 as U64;
unsafe extern "C" fn ZSTD_hash6(mut u: U64, mut h: U32, mut s: U64) -> size_t {
    return (((u << 64 as ::core::ffi::c_int - 48 as ::core::ffi::c_int).wrapping_mul(prime6bytes)
        ^ s)
        >> (64 as U32).wrapping_sub(h)) as size_t;
}
unsafe extern "C" fn ZSTD_hash6Ptr(mut p: *const ::core::ffi::c_void, mut h: U32) -> size_t {
    return ZSTD_hash6(MEM_readLE64(p), h, 0 as U64);
}
static mut prime8bytes: U64 = 0xcf1bbcdcb7a56463 as U64;
unsafe extern "C" fn ZSTD_hash8(mut u: U64, mut h: U32, mut s: U64) -> size_t {
    return ((u.wrapping_mul(prime8bytes) ^ s) >> (64 as U32).wrapping_sub(h)) as size_t;
}
unsafe extern "C" fn ZSTD_hash8Ptr(mut p: *const ::core::ffi::c_void, mut h: U32) -> size_t {
    return ZSTD_hash8(MEM_readLE64(p), h, 0 as U64);
}
unsafe extern "C" fn ERR_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return (code > -(ZSTD_error_maxCode as ::core::ffi::c_int) as size_t) as ::core::ffi::c_int
        as ::core::ffi::c_uint;
}
pub const ZDICT_DICTSIZE_MIN: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const FASTCOVER_MAX_F: ::core::ffi::c_int = 31 as ::core::ffi::c_int;
pub const FASTCOVER_MAX_ACCEL: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
pub const FASTCOVER_DEFAULT_SPLITPOINT: ::core::ffi::c_double = 0.75f64;
pub const DEFAULT_F: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
pub const DEFAULT_ACCEL: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
static mut g_displayLevel: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
static mut g_refreshRate: clock_t = CLOCKS_PER_SEC * 15 as clock_t / 100 as clock_t;
static mut g_time: clock_t = 0 as clock_t;
unsafe extern "C" fn FASTCOVER_hashPtrToIndex(
    mut p: *const ::core::ffi::c_void,
    mut f: U32,
    mut d: ::core::ffi::c_uint,
) -> size_t {
    if d == 6 as ::core::ffi::c_uint {
        return ZSTD_hash6Ptr(p, f);
    }
    return ZSTD_hash8Ptr(p, f);
}
static mut FASTCOVER_defaultAccelParameters: [FASTCOVER_accel_t; 11] = [
    FASTCOVER_accel_t {
        finalize: 100 as ::core::ffi::c_uint,
        skip: 0 as ::core::ffi::c_uint,
    },
    FASTCOVER_accel_t {
        finalize: 100 as ::core::ffi::c_uint,
        skip: 0 as ::core::ffi::c_uint,
    },
    FASTCOVER_accel_t {
        finalize: 50 as ::core::ffi::c_uint,
        skip: 1 as ::core::ffi::c_uint,
    },
    FASTCOVER_accel_t {
        finalize: 34 as ::core::ffi::c_uint,
        skip: 2 as ::core::ffi::c_uint,
    },
    FASTCOVER_accel_t {
        finalize: 25 as ::core::ffi::c_uint,
        skip: 3 as ::core::ffi::c_uint,
    },
    FASTCOVER_accel_t {
        finalize: 20 as ::core::ffi::c_uint,
        skip: 4 as ::core::ffi::c_uint,
    },
    FASTCOVER_accel_t {
        finalize: 17 as ::core::ffi::c_uint,
        skip: 5 as ::core::ffi::c_uint,
    },
    FASTCOVER_accel_t {
        finalize: 14 as ::core::ffi::c_uint,
        skip: 6 as ::core::ffi::c_uint,
    },
    FASTCOVER_accel_t {
        finalize: 13 as ::core::ffi::c_uint,
        skip: 7 as ::core::ffi::c_uint,
    },
    FASTCOVER_accel_t {
        finalize: 11 as ::core::ffi::c_uint,
        skip: 8 as ::core::ffi::c_uint,
    },
    FASTCOVER_accel_t {
        finalize: 10 as ::core::ffi::c_uint,
        skip: 9 as ::core::ffi::c_uint,
    },
];
unsafe extern "C" fn FASTCOVER_selectSegment(
    mut ctx: *const FASTCOVER_ctx_t,
    mut freqs: *mut U32,
    mut begin: U32,
    mut end: U32,
    mut parameters: ZDICT_cover_params_t,
    mut segmentFreqs: *mut U16,
) -> COVER_segment_t {
    let k: U32 = parameters.k as U32;
    let d: U32 = parameters.d as U32;
    let f: U32 = (*ctx).f as U32;
    let dmersInK: U32 = k.wrapping_sub(d).wrapping_add(1 as U32);
    let mut bestSegment: COVER_segment_t = COVER_segment_t {
        begin: 0 as U32,
        end: 0 as U32,
        score: 0 as U32,
    };
    let mut activeSegment: COVER_segment_t = COVER_segment_t {
        begin: 0,
        end: 0,
        score: 0,
    };
    activeSegment.begin = begin;
    activeSegment.end = begin;
    activeSegment.score = 0 as U32;
    while activeSegment.end < end {
        let idx: size_t = FASTCOVER_hashPtrToIndex(
            (*ctx).samples.offset(activeSegment.end as isize) as *const ::core::ffi::c_void,
            f,
            d as ::core::ffi::c_uint,
        ) as size_t;
        if *segmentFreqs.offset(idx as isize) as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
            activeSegment.score = (activeSegment.score as ::core::ffi::c_uint)
                .wrapping_add(*freqs.offset(idx as isize) as ::core::ffi::c_uint)
                as U32 as U32;
        }
        activeSegment.end = (activeSegment.end as ::core::ffi::c_uint)
            .wrapping_add(1 as ::core::ffi::c_uint) as U32 as U32;
        let ref mut fresh0 = *segmentFreqs.offset(idx as isize);
        *fresh0 = (*fresh0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as U16;
        if activeSegment.end.wrapping_sub(activeSegment.begin) == dmersInK.wrapping_add(1 as U32) {
            let delIndex: size_t = FASTCOVER_hashPtrToIndex(
                (*ctx).samples.offset(activeSegment.begin as isize) as *const ::core::ffi::c_void,
                f,
                d as ::core::ffi::c_uint,
            ) as size_t;
            let ref mut fresh1 = *segmentFreqs.offset(delIndex as isize);
            *fresh1 = (*fresh1 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as U16;
            if *segmentFreqs.offset(delIndex as isize) as ::core::ffi::c_int
                == 0 as ::core::ffi::c_int
            {
                activeSegment.score = (activeSegment.score as ::core::ffi::c_uint)
                    .wrapping_sub(*freqs.offset(delIndex as isize) as ::core::ffi::c_uint)
                    as U32 as U32;
            }
            activeSegment.begin = (activeSegment.begin as ::core::ffi::c_uint)
                .wrapping_add(1 as ::core::ffi::c_uint) as U32
                as U32;
        }
        if activeSegment.score > bestSegment.score {
            bestSegment = activeSegment;
        }
    }
    while activeSegment.begin < end {
        let delIndex_0: size_t = FASTCOVER_hashPtrToIndex(
            (*ctx).samples.offset(activeSegment.begin as isize) as *const ::core::ffi::c_void,
            f,
            d as ::core::ffi::c_uint,
        ) as size_t;
        let ref mut fresh2 = *segmentFreqs.offset(delIndex_0 as isize);
        *fresh2 = (*fresh2 as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as U16;
        activeSegment.begin = (activeSegment.begin as ::core::ffi::c_uint)
            .wrapping_add(1 as ::core::ffi::c_uint) as U32 as U32;
    }
    let mut pos: U32 = 0;
    pos = bestSegment.begin;
    while pos != bestSegment.end {
        let i: size_t = FASTCOVER_hashPtrToIndex(
            (*ctx).samples.offset(pos as isize) as *const ::core::ffi::c_void,
            f,
            d as ::core::ffi::c_uint,
        ) as size_t;
        *freqs.offset(i as isize) = 0 as U32;
        pos = pos.wrapping_add(1);
    }
    return bestSegment;
}
unsafe extern "C" fn FASTCOVER_checkParameters(
    mut parameters: ZDICT_cover_params_t,
    mut maxDictSize: size_t,
    mut f: ::core::ffi::c_uint,
    mut accel: ::core::ffi::c_uint,
) -> ::core::ffi::c_int {
    if parameters.d == 0 as ::core::ffi::c_uint || parameters.k == 0 as ::core::ffi::c_uint {
        return 0 as ::core::ffi::c_int;
    }
    if parameters.d != 6 as ::core::ffi::c_uint && parameters.d != 8 as ::core::ffi::c_uint {
        return 0 as ::core::ffi::c_int;
    }
    if parameters.k as size_t > maxDictSize {
        return 0 as ::core::ffi::c_int;
    }
    if parameters.d > parameters.k {
        return 0 as ::core::ffi::c_int;
    }
    if f > FASTCOVER_MAX_F as ::core::ffi::c_uint || f == 0 as ::core::ffi::c_uint {
        return 0 as ::core::ffi::c_int;
    }
    if parameters.splitPoint <= 0 as ::core::ffi::c_int as ::core::ffi::c_double
        || parameters.splitPoint > 1 as ::core::ffi::c_int as ::core::ffi::c_double
    {
        return 0 as ::core::ffi::c_int;
    }
    if accel > 10 as ::core::ffi::c_uint || accel == 0 as ::core::ffi::c_uint {
        return 0 as ::core::ffi::c_int;
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn FASTCOVER_ctx_destroy(mut ctx: *mut FASTCOVER_ctx_t) {
    if ctx.is_null() {
        return;
    }
    free((*ctx).freqs as *mut ::core::ffi::c_void);
    (*ctx).freqs = ::core::ptr::null_mut::<U32>();
    free((*ctx).offsets as *mut ::core::ffi::c_void);
    (*ctx).offsets = ::core::ptr::null_mut::<size_t>();
}
unsafe extern "C" fn FASTCOVER_computeFrequency(
    mut freqs: *mut U32,
    mut ctx: *const FASTCOVER_ctx_t,
) {
    let f: ::core::ffi::c_uint = (*ctx).f;
    let d: ::core::ffi::c_uint = (*ctx).d;
    let skip: ::core::ffi::c_uint = (*ctx).accelParams.skip;
    let readLength: ::core::ffi::c_uint = if d > 8 as ::core::ffi::c_uint {
        d
    } else {
        8 as ::core::ffi::c_uint
    };
    let mut i: size_t = 0;
    i = 0 as size_t;
    while i < (*ctx).nbTrainSamples {
        let mut start: size_t = *(*ctx).offsets.offset(i as isize);
        let currSampleEnd: size_t = *(*ctx).offsets.offset(i.wrapping_add(1 as size_t) as isize);
        while start.wrapping_add(readLength as size_t) <= currSampleEnd {
            let dmerIndex: size_t = FASTCOVER_hashPtrToIndex(
                (*ctx).samples.offset(start as isize) as *const ::core::ffi::c_void,
                f as U32,
                d,
            ) as size_t;
            let ref mut fresh3 = *freqs.offset(dmerIndex as isize);
            *fresh3 = (*fresh3).wrapping_add(1);
            start = start.wrapping_add(skip as size_t).wrapping_add(1 as size_t);
        }
        i = i.wrapping_add(1);
    }
}
unsafe extern "C" fn FASTCOVER_ctx_init(
    mut ctx: *mut FASTCOVER_ctx_t,
    mut samplesBuffer: *const ::core::ffi::c_void,
    mut samplesSizes: *const size_t,
    mut nbSamples: ::core::ffi::c_uint,
    mut d: ::core::ffi::c_uint,
    mut splitPoint: ::core::ffi::c_double,
    mut f: ::core::ffi::c_uint,
    mut accelParams: FASTCOVER_accel_t,
) -> size_t {
    let samples: *const BYTE = samplesBuffer as *const BYTE;
    let totalSamplesSize: size_t = COVER_sum(samplesSizes, nbSamples) as size_t;
    let nbTrainSamples: ::core::ffi::c_uint = if splitPoint < 1.0f64 {
        (nbSamples as ::core::ffi::c_double * splitPoint) as ::core::ffi::c_uint
    } else {
        nbSamples
    };
    let nbTestSamples: ::core::ffi::c_uint = if splitPoint < 1.0f64 {
        nbSamples.wrapping_sub(nbTrainSamples)
    } else {
        nbSamples
    };
    let trainingSamplesSize: size_t = if splitPoint < 1.0f64 {
        COVER_sum(samplesSizes, nbTrainSamples) as size_t
    } else {
        totalSamplesSize
    };
    let testSamplesSize: size_t = if splitPoint < 1.0f64 {
        COVER_sum(samplesSizes.offset(nbTrainSamples as isize), nbTestSamples) as size_t
    } else {
        totalSamplesSize
    };
    if totalSamplesSize
        < (if d as usize > ::core::mem::size_of::<U64>() as usize {
            d as usize
        } else {
            ::core::mem::size_of::<U64>() as usize
        })
        || totalSamplesSize
            >= (if ::core::mem::size_of::<size_t>() as usize == 8 as usize {
                -(1 as ::core::ffi::c_int) as ::core::ffi::c_uint
            } else {
                (1 as ::core::ffi::c_int as ::core::ffi::c_uint)
                    .wrapping_mul((1 as ::core::ffi::c_uint) << 30 as ::core::ffi::c_int)
            }) as size_t
    {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Total samples size is too large (%u MB), maximum size is %u MB\n\0" as *const u8
                    as *const ::core::ffi::c_char,
                (totalSamplesSize >> 20 as ::core::ffi::c_int) as ::core::ffi::c_uint,
                (if ::core::mem::size_of::<size_t>() as usize == 8 as usize {
                    -(1 as ::core::ffi::c_int) as ::core::ffi::c_uint
                } else {
                    (1 as ::core::ffi::c_int as ::core::ffi::c_uint)
                        .wrapping_mul((1 as ::core::ffi::c_uint) << 30 as ::core::ffi::c_int)
                }) >> 20 as ::core::ffi::c_int,
            );
            fflush(stderr);
        }
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if nbTrainSamples < 5 as ::core::ffi::c_uint {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Total number of training samples is %u and is invalid\n\0" as *const u8
                    as *const ::core::ffi::c_char,
                nbTrainSamples,
            );
            fflush(stderr);
        }
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if nbTestSamples < 1 as ::core::ffi::c_uint {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Total number of testing samples is %u and is invalid.\n\0" as *const u8
                    as *const ::core::ffi::c_char,
                nbTestSamples,
            );
            fflush(stderr);
        }
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    memset(
        ctx as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<FASTCOVER_ctx_t>() as size_t,
    );
    if g_displayLevel >= 2 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"Training on %u samples of total size %u\n\0" as *const u8
                as *const ::core::ffi::c_char,
            nbTrainSamples,
            trainingSamplesSize as ::core::ffi::c_uint,
        );
        fflush(stderr);
    }
    if g_displayLevel >= 2 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"Testing on %u samples of total size %u\n\0" as *const u8
                as *const ::core::ffi::c_char,
            nbTestSamples,
            testSamplesSize as ::core::ffi::c_uint,
        );
        fflush(stderr);
    }
    (*ctx).samples = samples;
    (*ctx).samplesSizes = samplesSizes;
    (*ctx).nbSamples = nbSamples as size_t;
    (*ctx).nbTrainSamples = nbTrainSamples as size_t;
    (*ctx).nbTestSamples = nbTestSamples as size_t;
    (*ctx).nbDmers = trainingSamplesSize
        .wrapping_sub(
            (if d as usize > ::core::mem::size_of::<U64>() as usize {
                d as size_t
            } else {
                ::core::mem::size_of::<U64>() as size_t
            }),
        )
        .wrapping_add(1 as size_t);
    (*ctx).d = d;
    (*ctx).f = f;
    (*ctx).accelParams = accelParams;
    (*ctx).offsets = calloc(
        nbSamples.wrapping_add(1 as ::core::ffi::c_uint) as size_t,
        ::core::mem::size_of::<size_t>() as size_t,
    ) as *mut size_t;
    if (*ctx).offsets.is_null() {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Failed to allocate scratch buffers \n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
        FASTCOVER_ctx_destroy(ctx);
        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
    }
    let mut i: U32 = 0;
    *(*ctx).offsets.offset(0 as ::core::ffi::c_int as isize) = 0 as size_t;
    i = 1 as U32;
    while i <= nbSamples as U32 {
        *(*ctx).offsets.offset(i as isize) =
            (*(*ctx).offsets.offset(i.wrapping_sub(1 as U32) as isize))
                .wrapping_add(*samplesSizes.offset(i.wrapping_sub(1 as U32) as isize));
        i = i.wrapping_add(1);
    }
    (*ctx).freqs = calloc(
        (1 as ::core::ffi::c_int as size_t) << f,
        ::core::mem::size_of::<U32>() as size_t,
    ) as *mut U32;
    if (*ctx).freqs.is_null() {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Failed to allocate frequency table \n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
        FASTCOVER_ctx_destroy(ctx);
        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
    }
    if g_displayLevel >= 2 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"Computing frequencies\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        fflush(stderr);
    }
    FASTCOVER_computeFrequency((*ctx).freqs, ctx);
    return 0 as size_t;
}
unsafe extern "C" fn FASTCOVER_buildDictionary(
    mut ctx: *const FASTCOVER_ctx_t,
    mut freqs: *mut U32,
    mut dictBuffer: *mut ::core::ffi::c_void,
    mut dictBufferCapacity: size_t,
    mut parameters: ZDICT_cover_params_t,
    mut segmentFreqs: *mut U16,
) -> size_t {
    let dict: *mut BYTE = dictBuffer as *mut BYTE;
    let mut tail: size_t = dictBufferCapacity;
    let epochs: COVER_epoch_info_t = COVER_computeEpochs(
        dictBufferCapacity as U32,
        (*ctx).nbDmers as U32,
        parameters.k as U32,
        1 as U32,
    ) as COVER_epoch_info_t;
    let maxZeroScoreRun: size_t = 10 as size_t;
    let mut zeroScoreRun: size_t = 0 as size_t;
    let mut epoch: size_t = 0;
    if g_displayLevel >= 2 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"Breaking content into %u epochs of size %u\n\0" as *const u8
                as *const ::core::ffi::c_char,
            epochs.num,
            epochs.size,
        );
        fflush(stderr);
    }
    epoch = 0 as size_t;
    while tail > 0 as size_t {
        let epochBegin: U32 = epoch.wrapping_mul(epochs.size as size_t) as U32;
        let epochEnd: U32 = epochBegin.wrapping_add(epochs.size);
        let mut segmentSize: size_t = 0;
        let mut segment: COVER_segment_t =
            FASTCOVER_selectSegment(ctx, freqs, epochBegin, epochEnd, parameters, segmentFreqs);
        if segment.score == 0 as U32 {
            zeroScoreRun = zeroScoreRun.wrapping_add(1);
            if zeroScoreRun >= maxZeroScoreRun {
                break;
            }
        } else {
            zeroScoreRun = 0 as size_t;
            segmentSize = if (segment
                .end
                .wrapping_sub(segment.begin)
                .wrapping_add(parameters.d as U32)
                .wrapping_sub(1 as U32) as size_t)
                < tail
            {
                segment
                    .end
                    .wrapping_sub(segment.begin)
                    .wrapping_add(parameters.d as U32)
                    .wrapping_sub(1 as U32) as size_t
            } else {
                tail
            };
            if segmentSize < parameters.d as size_t {
                break;
            }
            tail = (tail as ::core::ffi::c_ulong).wrapping_sub(segmentSize as ::core::ffi::c_ulong)
                as size_t as size_t;
            memcpy(
                dict.offset(tail as isize) as *mut ::core::ffi::c_void,
                (*ctx).samples.offset(segment.begin as isize) as *const ::core::ffi::c_void,
                segmentSize,
            );
            if g_displayLevel >= 2 as ::core::ffi::c_int {
                if clock() - g_time > g_refreshRate || g_displayLevel >= 4 as ::core::ffi::c_int {
                    g_time = clock();
                    fprintf(
                        stderr,
                        b"\r%u%%       \0" as *const u8 as *const ::core::ffi::c_char,
                        dictBufferCapacity
                            .wrapping_sub(tail)
                            .wrapping_mul(100 as size_t)
                            .wrapping_div(dictBufferCapacity)
                            as ::core::ffi::c_uint,
                    );
                    fflush(stderr);
                }
            }
        }
        epoch = epoch
            .wrapping_add(1 as size_t)
            .wrapping_rem(epochs.num as size_t);
    }
    if g_displayLevel >= 2 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"\r%79s\r\0" as *const u8 as *const ::core::ffi::c_char,
            b"\0" as *const u8 as *const ::core::ffi::c_char,
        );
        fflush(stderr);
    }
    return tail;
}
unsafe extern "C" fn FASTCOVER_tryParameters(mut opaque: *mut ::core::ffi::c_void) {
    let data: *mut FASTCOVER_tryParameters_data_t = opaque as *mut FASTCOVER_tryParameters_data_t;
    let ctx: *const FASTCOVER_ctx_t = (*data).ctx;
    let parameters: ZDICT_cover_params_t = (*data).parameters;
    let mut dictBufferCapacity: size_t = (*data).dictBufferCapacity;
    let mut totalCompressedSize: size_t = -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    let mut segmentFreqs: *mut U16 = calloc(
        (1 as ::core::ffi::c_int as size_t) << (*ctx).f,
        ::core::mem::size_of::<U16>() as size_t,
    ) as *mut U16;
    let dict: *mut BYTE = malloc(dictBufferCapacity) as *mut BYTE;
    let mut selection: COVER_dictSelection_t =
        COVER_dictSelectionError(-(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t);
    let mut freqs: *mut U32 = malloc(
        ((1 as ::core::ffi::c_int as size_t) << (*ctx).f)
            .wrapping_mul(::core::mem::size_of::<U32>() as size_t),
    ) as *mut U32;
    if segmentFreqs.is_null() || dict.is_null() || freqs.is_null() {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Failed to allocate buffers: out of memory\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
    } else {
        memcpy(
            freqs as *mut ::core::ffi::c_void,
            (*ctx).freqs as *const ::core::ffi::c_void,
            ((1 as ::core::ffi::c_int as size_t) << (*ctx).f)
                .wrapping_mul(::core::mem::size_of::<U32>() as size_t),
        );
        let tail: size_t = FASTCOVER_buildDictionary(
            ctx,
            freqs,
            dict as *mut ::core::ffi::c_void,
            dictBufferCapacity,
            parameters,
            segmentFreqs,
        ) as size_t;
        let nbFinalizeSamples: ::core::ffi::c_uint = (*ctx)
            .nbTrainSamples
            .wrapping_mul((*ctx).accelParams.finalize as size_t)
            .wrapping_div(100 as size_t)
            as ::core::ffi::c_uint;
        selection = COVER_selectDict(
            dict.offset(tail as isize),
            dictBufferCapacity,
            dictBufferCapacity.wrapping_sub(tail),
            (*ctx).samples,
            (*ctx).samplesSizes,
            nbFinalizeSamples,
            (*ctx).nbTrainSamples,
            (*ctx).nbSamples,
            parameters,
            (*ctx).offsets,
            totalCompressedSize,
        );
        if COVER_dictSelectionIsError(selection) != 0 {
            if g_displayLevel >= 1 as ::core::ffi::c_int {
                fprintf(
                    stderr,
                    b"Failed to select dictionary\n\0" as *const u8 as *const ::core::ffi::c_char,
                );
                fflush(stderr);
            }
        }
    }
    free(dict as *mut ::core::ffi::c_void);
    COVER_best_finish((*data).best, parameters, selection);
    free(data as *mut ::core::ffi::c_void);
    free(segmentFreqs as *mut ::core::ffi::c_void);
    COVER_dictSelectionFree(selection);
    free(freqs as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn FASTCOVER_convertToCoverParams(
    mut fastCoverParams: ZDICT_fastCover_params_t,
    mut coverParams: *mut ZDICT_cover_params_t,
) {
    (*coverParams).k = fastCoverParams.k;
    (*coverParams).d = fastCoverParams.d;
    (*coverParams).steps = fastCoverParams.steps;
    (*coverParams).nbThreads = fastCoverParams.nbThreads;
    (*coverParams).splitPoint = fastCoverParams.splitPoint;
    (*coverParams).zParams = fastCoverParams.zParams;
    (*coverParams).shrinkDict = fastCoverParams.shrinkDict;
}
unsafe extern "C" fn FASTCOVER_convertToFastCoverParams(
    mut coverParams: ZDICT_cover_params_t,
    mut fastCoverParams: *mut ZDICT_fastCover_params_t,
    mut f: ::core::ffi::c_uint,
    mut accel: ::core::ffi::c_uint,
) {
    (*fastCoverParams).k = coverParams.k;
    (*fastCoverParams).d = coverParams.d;
    (*fastCoverParams).steps = coverParams.steps;
    (*fastCoverParams).nbThreads = coverParams.nbThreads;
    (*fastCoverParams).splitPoint = coverParams.splitPoint;
    (*fastCoverParams).f = f;
    (*fastCoverParams).accel = accel;
    (*fastCoverParams).zParams = coverParams.zParams;
    (*fastCoverParams).shrinkDict = coverParams.shrinkDict;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_trainFromBuffer_fastCover(
    mut dictBuffer: *mut ::core::ffi::c_void,
    mut dictBufferCapacity: size_t,
    mut samplesBuffer: *const ::core::ffi::c_void,
    mut samplesSizes: *const size_t,
    mut nbSamples: ::core::ffi::c_uint,
    mut parameters: ZDICT_fastCover_params_t,
) -> size_t {
    let dict: *mut BYTE = dictBuffer as *mut BYTE;
    let mut ctx: FASTCOVER_ctx_t = FASTCOVER_ctx_t {
        samples: ::core::ptr::null::<BYTE>(),
        offsets: ::core::ptr::null_mut::<size_t>(),
        samplesSizes: ::core::ptr::null::<size_t>(),
        nbSamples: 0,
        nbTrainSamples: 0,
        nbTestSamples: 0,
        nbDmers: 0,
        freqs: ::core::ptr::null_mut::<U32>(),
        d: 0,
        f: 0,
        accelParams: FASTCOVER_accel_t {
            finalize: 0,
            skip: 0,
        },
    };
    let mut coverParams: ZDICT_cover_params_t = ZDICT_cover_params_t {
        k: 0,
        d: 0,
        steps: 0,
        nbThreads: 0,
        splitPoint: 0.,
        shrinkDict: 0,
        shrinkDictMaxRegression: 0,
        zParams: ZDICT_params_t {
            compressionLevel: 0,
            notificationLevel: 0,
            dictID: 0,
        },
    };
    let mut accelParams: FASTCOVER_accel_t = FASTCOVER_accel_t {
        finalize: 0,
        skip: 0,
    };
    g_displayLevel = parameters.zParams.notificationLevel as ::core::ffi::c_int;
    parameters.splitPoint = 1.0f64;
    parameters.f = if parameters.f == 0 as ::core::ffi::c_uint {
        DEFAULT_F as ::core::ffi::c_uint
    } else {
        parameters.f
    };
    parameters.accel = if parameters.accel == 0 as ::core::ffi::c_uint {
        DEFAULT_ACCEL as ::core::ffi::c_uint
    } else {
        parameters.accel
    };
    memset(
        &raw mut coverParams as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<ZDICT_cover_params_t>() as size_t,
    );
    FASTCOVER_convertToCoverParams(parameters, &raw mut coverParams);
    if FASTCOVER_checkParameters(
        coverParams,
        dictBufferCapacity,
        parameters.f,
        parameters.accel,
    ) == 0
    {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"FASTCOVER parameters incorrect\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
        return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
    }
    if nbSamples == 0 as ::core::ffi::c_uint {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"FASTCOVER must have at least one input file\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN as size_t {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"dictBufferCapacity must be at least %u\n\0" as *const u8
                    as *const ::core::ffi::c_char,
                256 as ::core::ffi::c_int,
            );
            fflush(stderr);
        }
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    accelParams = FASTCOVER_defaultAccelParameters[parameters.accel as usize];
    let initVal: size_t = FASTCOVER_ctx_init(
        &raw mut ctx,
        samplesBuffer,
        samplesSizes,
        nbSamples,
        coverParams.d,
        parameters.splitPoint,
        parameters.f,
        accelParams,
    ) as size_t;
    if ERR_isError(initVal) != 0 {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Failed to initialize context\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
        return initVal;
    }
    COVER_warnOnSmallCorpus(dictBufferCapacity, ctx.nbDmers, g_displayLevel);
    if g_displayLevel >= 2 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"Building dictionary\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        fflush(stderr);
    }
    let mut segmentFreqs: *mut U16 = calloc(
        (1 as ::core::ffi::c_int as size_t) << parameters.f,
        ::core::mem::size_of::<U16>() as size_t,
    ) as *mut U16;
    let tail: size_t = FASTCOVER_buildDictionary(
        &raw mut ctx,
        ctx.freqs,
        dictBuffer,
        dictBufferCapacity,
        coverParams,
        segmentFreqs,
    ) as size_t;
    let nbFinalizeSamples: ::core::ffi::c_uint =
        ctx.nbTrainSamples
            .wrapping_mul(ctx.accelParams.finalize as size_t)
            .wrapping_div(100 as size_t) as ::core::ffi::c_uint;
    let dictionarySize: size_t = ZDICT_finalizeDictionary(
        dict as *mut ::core::ffi::c_void,
        dictBufferCapacity,
        dict.offset(tail as isize) as *const ::core::ffi::c_void,
        dictBufferCapacity.wrapping_sub(tail),
        samplesBuffer,
        samplesSizes,
        nbFinalizeSamples,
        coverParams.zParams,
    ) as size_t;
    if ERR_isError(dictionarySize) == 0 {
        if g_displayLevel >= 2 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Constructed dictionary of size %u\n\0" as *const u8 as *const ::core::ffi::c_char,
                dictionarySize as ::core::ffi::c_uint,
            );
            fflush(stderr);
        }
    }
    FASTCOVER_ctx_destroy(&raw mut ctx);
    free(segmentFreqs as *mut ::core::ffi::c_void);
    return dictionarySize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_optimizeTrainFromBuffer_fastCover(
    mut dictBuffer: *mut ::core::ffi::c_void,
    mut dictBufferCapacity: size_t,
    mut samplesBuffer: *const ::core::ffi::c_void,
    mut samplesSizes: *const size_t,
    mut nbSamples: ::core::ffi::c_uint,
    mut parameters: *mut ZDICT_fastCover_params_t,
) -> size_t {
    let mut coverParams: ZDICT_cover_params_t = ZDICT_cover_params_t {
        k: 0,
        d: 0,
        steps: 0,
        nbThreads: 0,
        splitPoint: 0.,
        shrinkDict: 0,
        shrinkDictMaxRegression: 0,
        zParams: ZDICT_params_t {
            compressionLevel: 0,
            notificationLevel: 0,
            dictID: 0,
        },
    };
    let mut accelParams: FASTCOVER_accel_t = FASTCOVER_accel_t {
        finalize: 0,
        skip: 0,
    };
    let nbThreads: ::core::ffi::c_uint = (*parameters).nbThreads;
    let splitPoint: ::core::ffi::c_double = if (*parameters).splitPoint <= 0.0f64 {
        FASTCOVER_DEFAULT_SPLITPOINT
    } else {
        (*parameters).splitPoint
    };
    let kMinD: ::core::ffi::c_uint = if (*parameters).d == 0 as ::core::ffi::c_uint {
        6 as ::core::ffi::c_uint
    } else {
        (*parameters).d
    };
    let kMaxD: ::core::ffi::c_uint = if (*parameters).d == 0 as ::core::ffi::c_uint {
        8 as ::core::ffi::c_uint
    } else {
        (*parameters).d
    };
    let kMinK: ::core::ffi::c_uint = if (*parameters).k == 0 as ::core::ffi::c_uint {
        50 as ::core::ffi::c_uint
    } else {
        (*parameters).k
    };
    let kMaxK: ::core::ffi::c_uint = if (*parameters).k == 0 as ::core::ffi::c_uint {
        2000 as ::core::ffi::c_uint
    } else {
        (*parameters).k
    };
    let kSteps: ::core::ffi::c_uint = if (*parameters).steps == 0 as ::core::ffi::c_uint {
        40 as ::core::ffi::c_uint
    } else {
        (*parameters).steps
    };
    let kStepSize: ::core::ffi::c_uint =
        if kMaxK.wrapping_sub(kMinK).wrapping_div(kSteps) > 1 as ::core::ffi::c_uint {
            kMaxK.wrapping_sub(kMinK).wrapping_div(kSteps)
        } else {
            1 as ::core::ffi::c_uint
        };
    let kIterations: ::core::ffi::c_uint = (1 as ::core::ffi::c_uint)
        .wrapping_add(
            kMaxD
                .wrapping_sub(kMinD)
                .wrapping_div(2 as ::core::ffi::c_uint),
        )
        .wrapping_mul(
            (1 as ::core::ffi::c_uint)
                .wrapping_add(kMaxK.wrapping_sub(kMinK).wrapping_div(kStepSize)),
        );
    let f: ::core::ffi::c_uint = if (*parameters).f == 0 as ::core::ffi::c_uint {
        DEFAULT_F as ::core::ffi::c_uint
    } else {
        (*parameters).f
    };
    let accel: ::core::ffi::c_uint = if (*parameters).accel == 0 as ::core::ffi::c_uint {
        DEFAULT_ACCEL as ::core::ffi::c_uint
    } else {
        (*parameters).accel
    };
    let shrinkDict: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let displayLevel: ::core::ffi::c_int =
        (*parameters).zParams.notificationLevel as ::core::ffi::c_int;
    let mut iteration: ::core::ffi::c_uint = 1 as ::core::ffi::c_uint;
    let mut d: ::core::ffi::c_uint = 0;
    let mut k: ::core::ffi::c_uint = 0;
    let mut best: COVER_best_t = COVER_best_t {
        mutex: 0,
        cond: 0,
        liveJobs: 0,
        dict: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        dictSize: 0,
        parameters: ZDICT_cover_params_t {
            k: 0,
            d: 0,
            steps: 0,
            nbThreads: 0,
            splitPoint: 0.,
            shrinkDict: 0,
            shrinkDictMaxRegression: 0,
            zParams: ZDICT_params_t {
                compressionLevel: 0,
                notificationLevel: 0,
                dictID: 0,
            },
        },
        compressedSize: 0,
    };
    let mut pool: *mut POOL_ctx = ::core::ptr::null_mut::<POOL_ctx>();
    let mut warned: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if splitPoint <= 0 as ::core::ffi::c_int as ::core::ffi::c_double
        || splitPoint > 1 as ::core::ffi::c_int as ::core::ffi::c_double
    {
        if displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Incorrect splitPoint\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
        return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
    }
    if accel == 0 as ::core::ffi::c_uint || accel > FASTCOVER_MAX_ACCEL as ::core::ffi::c_uint {
        if displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Incorrect accel\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
        return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
    }
    if kMinK < kMaxD || kMaxK < kMinK {
        if displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Incorrect k\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
        return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
    }
    if nbSamples == 0 as ::core::ffi::c_uint {
        if displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"FASTCOVER must have at least one input file\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if dictBufferCapacity < ZDICT_DICTSIZE_MIN as size_t {
        if displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"dictBufferCapacity must be at least %u\n\0" as *const u8
                    as *const ::core::ffi::c_char,
                256 as ::core::ffi::c_int,
            );
            fflush(stderr);
        }
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if nbThreads > 1 as ::core::ffi::c_uint {
        pool = POOL_create(nbThreads as size_t, 1 as size_t);
        if pool.is_null() {
            return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
        }
    }
    COVER_best_init(&raw mut best);
    memset(
        &raw mut coverParams as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<ZDICT_cover_params_t>() as size_t,
    );
    FASTCOVER_convertToCoverParams(*parameters, &raw mut coverParams);
    accelParams = FASTCOVER_defaultAccelParameters[accel as usize];
    g_displayLevel = if displayLevel == 0 as ::core::ffi::c_int {
        0 as ::core::ffi::c_int
    } else {
        displayLevel - 1 as ::core::ffi::c_int
    };
    if displayLevel >= 2 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"Trying %u different sets of parameters\n\0" as *const u8
                as *const ::core::ffi::c_char,
            kIterations,
        );
        fflush(stderr);
    }
    d = kMinD;
    while d <= kMaxD {
        let mut ctx: FASTCOVER_ctx_t = FASTCOVER_ctx_t {
            samples: ::core::ptr::null::<BYTE>(),
            offsets: ::core::ptr::null_mut::<size_t>(),
            samplesSizes: ::core::ptr::null::<size_t>(),
            nbSamples: 0,
            nbTrainSamples: 0,
            nbTestSamples: 0,
            nbDmers: 0,
            freqs: ::core::ptr::null_mut::<U32>(),
            d: 0,
            f: 0,
            accelParams: FASTCOVER_accel_t {
                finalize: 0,
                skip: 0,
            },
        };
        if displayLevel >= 3 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"d=%u\n\0" as *const u8 as *const ::core::ffi::c_char,
                d,
            );
            fflush(stderr);
        }
        let initVal: size_t = FASTCOVER_ctx_init(
            &raw mut ctx,
            samplesBuffer,
            samplesSizes,
            nbSamples,
            d,
            splitPoint,
            f,
            accelParams,
        ) as size_t;
        if ERR_isError(initVal) != 0 {
            if displayLevel >= 1 as ::core::ffi::c_int {
                fprintf(
                    stderr,
                    b"Failed to initialize context\n\0" as *const u8 as *const ::core::ffi::c_char,
                );
                fflush(stderr);
            }
            COVER_best_destroy(&raw mut best);
            POOL_free(pool);
            return initVal;
        }
        if warned == 0 {
            COVER_warnOnSmallCorpus(dictBufferCapacity, ctx.nbDmers, displayLevel);
            warned = 1 as ::core::ffi::c_int;
        }
        k = kMinK;
        while k <= kMaxK {
            let mut data: *mut FASTCOVER_tryParameters_data_t =
                malloc(::core::mem::size_of::<FASTCOVER_tryParameters_data_t>() as size_t)
                    as *mut FASTCOVER_tryParameters_data_t;
            if displayLevel >= 3 as ::core::ffi::c_int {
                fprintf(
                    stderr,
                    b"k=%u\n\0" as *const u8 as *const ::core::ffi::c_char,
                    k,
                );
                fflush(stderr);
            }
            if data.is_null() {
                if displayLevel >= 1 as ::core::ffi::c_int {
                    fprintf(
                        stderr,
                        b"Failed to allocate parameters\n\0" as *const u8
                            as *const ::core::ffi::c_char,
                    );
                    fflush(stderr);
                }
                COVER_best_destroy(&raw mut best);
                FASTCOVER_ctx_destroy(&raw mut ctx);
                POOL_free(pool);
                return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
            }
            (*data).ctx = &raw mut ctx;
            (*data).best = &raw mut best;
            (*data).dictBufferCapacity = dictBufferCapacity;
            (*data).parameters = coverParams;
            (*data).parameters.k = k;
            (*data).parameters.d = d;
            (*data).parameters.splitPoint = splitPoint;
            (*data).parameters.steps = kSteps;
            (*data).parameters.shrinkDict = shrinkDict;
            (*data).parameters.zParams.notificationLevel = g_displayLevel as ::core::ffi::c_uint;
            if FASTCOVER_checkParameters(
                (*data).parameters,
                dictBufferCapacity,
                (*(*data).ctx).f,
                accel,
            ) == 0
            {
                if g_displayLevel >= 1 as ::core::ffi::c_int {
                    fprintf(
                        stderr,
                        b"FASTCOVER parameters incorrect\n\0" as *const u8
                            as *const ::core::ffi::c_char,
                    );
                    fflush(stderr);
                }
                free(data as *mut ::core::ffi::c_void);
            } else {
                COVER_best_start(&raw mut best);
                if !pool.is_null() {
                    POOL_add(
                        pool,
                        Some(
                            FASTCOVER_tryParameters
                                as unsafe extern "C" fn(*mut ::core::ffi::c_void) -> (),
                        ),
                        data as *mut ::core::ffi::c_void,
                    );
                } else {
                    FASTCOVER_tryParameters(data as *mut ::core::ffi::c_void);
                }
                if displayLevel >= 2 as ::core::ffi::c_int {
                    if clock() - g_time > g_refreshRate || displayLevel >= 4 as ::core::ffi::c_int {
                        g_time = clock();
                        fprintf(
                            stderr,
                            b"\r%u%%       \0" as *const u8 as *const ::core::ffi::c_char,
                            iteration
                                .wrapping_mul(100 as ::core::ffi::c_uint)
                                .wrapping_div(kIterations),
                        );
                        fflush(stderr);
                    }
                }
                iteration = iteration.wrapping_add(1);
            }
            k = k.wrapping_add(kStepSize);
        }
        COVER_best_wait(&raw mut best);
        FASTCOVER_ctx_destroy(&raw mut ctx);
        d = d.wrapping_add(2 as ::core::ffi::c_uint);
    }
    if displayLevel >= 2 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"\r%79s\r\0" as *const u8 as *const ::core::ffi::c_char,
            b"\0" as *const u8 as *const ::core::ffi::c_char,
        );
        fflush(stderr);
    }
    let dictSize: size_t = best.dictSize;
    if ERR_isError(best.compressedSize) != 0 {
        let compressedSize: size_t = best.compressedSize;
        COVER_best_destroy(&raw mut best);
        POOL_free(pool);
        return compressedSize;
    }
    FASTCOVER_convertToFastCoverParams(best.parameters, parameters, f, accel);
    memcpy(dictBuffer, best.dict, dictSize);
    COVER_best_destroy(&raw mut best);
    POOL_free(pool);
    return dictSize;
}
