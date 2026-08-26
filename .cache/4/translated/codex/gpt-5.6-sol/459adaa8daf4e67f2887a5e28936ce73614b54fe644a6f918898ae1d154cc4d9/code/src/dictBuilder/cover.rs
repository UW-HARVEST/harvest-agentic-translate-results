extern "C" {
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    pub type ZSTD_CCtx_s;
    pub type ZSTD_CDict_s;
    pub type POOL_ctx_s;
    static mut stderr: *mut FILE;
    fn fflush(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn qsort_r(
        __base: *mut ::core::ffi::c_void,
        __nmemb: size_t,
        __size: size_t,
        __compar: __compar_d_fn_t,
        __arg: *mut ::core::ffi::c_void,
    );
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
    fn memcmp(
        __s1: *const ::core::ffi::c_void,
        __s2: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn clock() -> clock_t;
    fn ZSTD_compressBound(srcSize: size_t) -> size_t;
    fn ZSTD_createCCtx() -> *mut ZSTD_CCtx;
    fn ZSTD_freeCCtx(cctx: *mut ZSTD_CCtx) -> size_t;
    fn ZSTD_createCDict(
        dictBuffer: *const ::core::ffi::c_void,
        dictSize: size_t,
        compressionLevel: ::core::ffi::c_int,
    ) -> *mut ZSTD_CDict;
    fn ZSTD_freeCDict(CDict: *mut ZSTD_CDict) -> size_t;
    fn ZSTD_compress_usingCDict(
        cctx: *mut ZSTD_CCtx,
        dst: *mut ::core::ffi::c_void,
        dstCapacity: size_t,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        cdict: *const ZSTD_CDict,
    ) -> size_t;
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
    fn ZDICT_isError(errorCode: size_t) -> ::core::ffi::c_uint;
}
pub type size_t = usize;
pub type __uint8_t = u8;
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
pub type __compar_d_fn_t = Option<
    unsafe extern "C" fn(
        *const ::core::ffi::c_void,
        *const ::core::ffi::c_void,
        *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int,
>;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type BYTE = uint8_t;
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
pub type ZSTD_CCtx = ZSTD_CCtx_s;
pub type ZSTD_CDict = ZSTD_CDict_s;
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
pub type COVER_map_t = COVER_map_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct COVER_map_s {
    pub data: *mut COVER_map_pair_t,
    pub sizeLog: U32,
    pub size: U32,
    pub sizeMask: U32,
}
pub type COVER_map_pair_t = COVER_map_pair_t_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct COVER_map_pair_t_s {
    pub key: U32,
    pub value: U32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct COVER_ctx_t {
    pub samples: *const BYTE,
    pub offsets: *mut size_t,
    pub samplesSizes: *const size_t,
    pub nbSamples: size_t,
    pub nbTrainSamples: size_t,
    pub nbTestSamples: size_t,
    pub suffix: *mut U32,
    pub suffixSize: size_t,
    pub freqs: *mut U32,
    pub dmerAt: *mut U32,
    pub d: ::core::ffi::c_uint,
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
pub type COVER_tryParameters_data_t = COVER_tryParameters_data_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct COVER_tryParameters_data_s {
    pub ctx: *const COVER_ctx_t,
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
unsafe extern "C" fn ERR_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return (code > -(ZSTD_error_maxCode as ::core::ffi::c_int) as size_t) as ::core::ffi::c_int
        as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn ZSTD_countLeadingZeros32(mut val: U32) -> ::core::ffi::c_uint {
    return val.leading_zeros() as i32 as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn ZSTD_highbit32(mut val: U32) -> ::core::ffi::c_uint {
    return (31 as ::core::ffi::c_uint).wrapping_sub(ZSTD_countLeadingZeros32(val));
}
pub const ZDICT_DICTSIZE_MIN: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const COVER_DEFAULT_SPLITPOINT: ::core::ffi::c_double = 1.0f64;
static mut g_displayLevel: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
static mut g_refreshRate: clock_t = CLOCKS_PER_SEC * 15 as clock_t / 100 as clock_t;
static mut g_time: clock_t = 0 as clock_t;
pub const MAP_EMPTY_VALUE: U32 = -(1 as ::core::ffi::c_int) as U32;
unsafe extern "C" fn COVER_map_clear(mut map: *mut COVER_map_t) {
    memset(
        (*map).data as *mut ::core::ffi::c_void,
        MAP_EMPTY_VALUE as ::core::ffi::c_int,
        ((*map).size as size_t).wrapping_mul(::core::mem::size_of::<COVER_map_pair_t>() as size_t),
    );
}
unsafe extern "C" fn COVER_map_init(
    mut map: *mut COVER_map_t,
    mut size: U32,
) -> ::core::ffi::c_int {
    (*map).sizeLog = ZSTD_highbit32(size).wrapping_add(2 as ::core::ffi::c_uint) as U32;
    (*map).size = (1 as ::core::ffi::c_int as U32) << (*map).sizeLog;
    (*map).sizeMask = (*map).size.wrapping_sub(1 as U32);
    (*map).data = malloc(
        ((*map).size as size_t).wrapping_mul(::core::mem::size_of::<COVER_map_pair_t>() as size_t),
    ) as *mut COVER_map_pair_t;
    if (*map).data.is_null() {
        (*map).sizeLog = 0 as U32;
        (*map).size = 0 as U32;
        return 0 as ::core::ffi::c_int;
    }
    COVER_map_clear(map);
    return 1 as ::core::ffi::c_int;
}
static mut COVER_prime4bytes: U32 = 2654435761 as U32;
unsafe extern "C" fn COVER_map_hash(mut map: *mut COVER_map_t, mut key: U32) -> U32 {
    return key.wrapping_mul(COVER_prime4bytes) >> (32 as U32).wrapping_sub((*map).sizeLog);
}
unsafe extern "C" fn COVER_map_index(mut map: *mut COVER_map_t, mut key: U32) -> U32 {
    let hash: U32 = COVER_map_hash(map, key) as U32;
    let mut i: U32 = 0;
    i = hash;
    loop {
        let mut pos: *mut COVER_map_pair_t =
            (*map).data.offset(i as isize) as *mut COVER_map_pair_t;
        if (*pos).value == MAP_EMPTY_VALUE {
            return i;
        }
        if (*pos).key == key {
            return i;
        }
        i = i.wrapping_add(1 as U32) & (*map).sizeMask;
    }
}
unsafe extern "C" fn COVER_map_at(mut map: *mut COVER_map_t, mut key: U32) -> *mut U32 {
    let mut pos: *mut COVER_map_pair_t = (*map).data.offset((COVER_map_index
        as unsafe extern "C" fn(*mut COVER_map_t, U32) -> U32)(
        map, key
    ) as isize) as *mut COVER_map_pair_t;
    if (*pos).value == MAP_EMPTY_VALUE {
        (*pos).key = key;
        (*pos).value = 0 as U32;
    }
    return &raw mut (*pos).value;
}
unsafe extern "C" fn COVER_map_remove(mut map: *mut COVER_map_t, mut key: U32) {
    let mut i: U32 = COVER_map_index(map, key);
    let mut del: *mut COVER_map_pair_t = (*map).data.offset(i as isize) as *mut COVER_map_pair_t;
    let mut shift: U32 = 1 as U32;
    if (*del).value == MAP_EMPTY_VALUE {
        return;
    }
    i = i.wrapping_add(1 as U32) & (*map).sizeMask;
    loop {
        let pos: *mut COVER_map_pair_t = (*map).data.offset(i as isize) as *mut COVER_map_pair_t;
        if (*pos).value == MAP_EMPTY_VALUE {
            (*del).value = MAP_EMPTY_VALUE;
            return;
        }
        if i.wrapping_sub(COVER_map_hash(map, (*pos).key)) & (*map).sizeMask >= shift {
            (*del).key = (*pos).key;
            (*del).value = (*pos).value;
            del = pos;
            shift = 1 as U32;
        } else {
            shift = shift.wrapping_add(1);
        }
        i = i.wrapping_add(1 as U32) & (*map).sizeMask;
    }
}
unsafe extern "C" fn COVER_map_destroy(mut map: *mut COVER_map_t) {
    if !(*map).data.is_null() {
        free((*map).data as *mut ::core::ffi::c_void);
    }
    (*map).data = ::core::ptr::null_mut::<COVER_map_pair_t>();
    (*map).size = 0 as U32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_sum(
    mut samplesSizes: *const size_t,
    mut nbSamples: ::core::ffi::c_uint,
) -> size_t {
    let mut sum: size_t = 0 as size_t;
    let mut i: ::core::ffi::c_uint = 0;
    i = 0 as ::core::ffi::c_uint;
    while i < nbSamples {
        sum = (sum as ::core::ffi::c_ulong)
            .wrapping_add(*samplesSizes.offset(i as isize) as ::core::ffi::c_ulong)
            as size_t as size_t;
        i = i.wrapping_add(1);
    }
    return sum;
}
unsafe extern "C" fn COVER_cmp(
    mut ctx: *mut COVER_ctx_t,
    mut lp: *const ::core::ffi::c_void,
    mut rp: *const ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    let lhs: U32 = *(lp as *const U32);
    let rhs: U32 = *(rp as *const U32);
    return memcmp(
        (*ctx).samples.offset(lhs as isize) as *const ::core::ffi::c_void,
        (*ctx).samples.offset(rhs as isize) as *const ::core::ffi::c_void,
        (*ctx).d as size_t,
    );
}
unsafe extern "C" fn COVER_cmp8(
    mut ctx: *mut COVER_ctx_t,
    mut lp: *const ::core::ffi::c_void,
    mut rp: *const ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    let mask: U64 = if (*ctx).d == 8 as ::core::ffi::c_uint {
        -(1 as ::core::ffi::c_int) as U64
    } else {
        ((1 as ::core::ffi::c_int as U64) << (8 as ::core::ffi::c_uint).wrapping_mul((*ctx).d))
            .wrapping_sub(1 as U64)
    };
    let lhs: U64 = MEM_readLE64(
        (*ctx).samples.offset(*(lp as *const U32) as isize) as *const ::core::ffi::c_void
    ) as U64
        & mask;
    let rhs: U64 = MEM_readLE64(
        (*ctx).samples.offset(*(rp as *const U32) as isize) as *const ::core::ffi::c_void
    ) as U64
        & mask;
    if lhs < rhs {
        return -(1 as ::core::ffi::c_int);
    }
    return (lhs > rhs) as ::core::ffi::c_int;
}
unsafe extern "C" fn COVER_strict_cmp(
    mut lp: *const ::core::ffi::c_void,
    mut rp: *const ::core::ffi::c_void,
    mut g_coverCtx: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = COVER_cmp(g_coverCtx as *mut COVER_ctx_t, lp, rp);
    if result == 0 as ::core::ffi::c_int {
        result = if lp < rp {
            -(1 as ::core::ffi::c_int)
        } else {
            1 as ::core::ffi::c_int
        };
    }
    return result;
}
unsafe extern "C" fn COVER_strict_cmp8(
    mut lp: *const ::core::ffi::c_void,
    mut rp: *const ::core::ffi::c_void,
    mut g_coverCtx: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = COVER_cmp8(g_coverCtx as *mut COVER_ctx_t, lp, rp);
    if result == 0 as ::core::ffi::c_int {
        result = if lp < rp {
            -(1 as ::core::ffi::c_int)
        } else {
            1 as ::core::ffi::c_int
        };
    }
    return result;
}
unsafe extern "C" fn stableSort(mut ctx: *mut COVER_ctx_t) {
    qsort_r(
        (*ctx).suffix as *mut ::core::ffi::c_void,
        (*ctx).suffixSize,
        ::core::mem::size_of::<U32>() as size_t,
        if (*ctx).d <= 8 as ::core::ffi::c_uint {
            Some(
                COVER_strict_cmp8
                    as unsafe extern "C" fn(
                        *const ::core::ffi::c_void,
                        *const ::core::ffi::c_void,
                        *mut ::core::ffi::c_void,
                    ) -> ::core::ffi::c_int,
            )
        } else {
            Some(
                COVER_strict_cmp
                    as unsafe extern "C" fn(
                        *const ::core::ffi::c_void,
                        *const ::core::ffi::c_void,
                        *mut ::core::ffi::c_void,
                    ) -> ::core::ffi::c_int,
            )
        },
        ctx as *mut ::core::ffi::c_void,
    );
}
unsafe extern "C" fn COVER_lower_bound(
    mut first: *const size_t,
    mut last: *const size_t,
    mut value: size_t,
) -> *const size_t {
    let mut count: size_t = last.offset_from(first) as ::core::ffi::c_long as size_t;
    while count != 0 as size_t {
        let mut step: size_t = count.wrapping_div(2 as size_t);
        let mut ptr: *const size_t = first;
        ptr = ptr.offset(step as isize);
        if *ptr < value {
            ptr = ptr.offset(1);
            first = ptr;
            count = (count as ::core::ffi::c_ulong)
                .wrapping_sub(step.wrapping_add(1 as size_t) as ::core::ffi::c_ulong)
                as size_t as size_t;
        } else {
            count = step;
        }
    }
    return first;
}
unsafe extern "C" fn COVER_groupBy(
    mut data: *const ::core::ffi::c_void,
    mut count: size_t,
    mut size: size_t,
    mut ctx: *mut COVER_ctx_t,
    mut cmp: Option<
        unsafe extern "C" fn(
            *mut COVER_ctx_t,
            *const ::core::ffi::c_void,
            *const ::core::ffi::c_void,
        ) -> ::core::ffi::c_int,
    >,
    mut grp: Option<
        unsafe extern "C" fn(
            *mut COVER_ctx_t,
            *const ::core::ffi::c_void,
            *const ::core::ffi::c_void,
        ) -> (),
    >,
) {
    let mut ptr: *const BYTE = data as *const BYTE;
    let mut num: size_t = 0 as size_t;
    while num < count {
        let mut grpEnd: *const BYTE = ptr.offset(size as isize);
        num = num.wrapping_add(1);
        while num < count
            && cmp.expect("non-null function pointer")(
                ctx,
                ptr as *const ::core::ffi::c_void,
                grpEnd as *const ::core::ffi::c_void,
            ) == 0 as ::core::ffi::c_int
        {
            grpEnd = grpEnd.offset(size as isize);
            num = num.wrapping_add(1);
        }
        grp.expect("non-null function pointer")(
            ctx,
            ptr as *const ::core::ffi::c_void,
            grpEnd as *const ::core::ffi::c_void,
        );
        ptr = grpEnd;
    }
}
unsafe extern "C" fn COVER_group(
    mut ctx: *mut COVER_ctx_t,
    mut group: *const ::core::ffi::c_void,
    mut groupEnd: *const ::core::ffi::c_void,
) {
    let mut grpPtr: *const U32 = group as *const U32;
    let mut grpEnd: *const U32 = groupEnd as *const U32;
    let dmerId: U32 = grpPtr.offset_from((*ctx).suffix) as ::core::ffi::c_long as U32;
    let mut freq: U32 = 0 as U32;
    let mut curOffsetPtr: *const size_t = (*ctx).offsets;
    let mut offsetsEnd: *const size_t = (*ctx).offsets.offset((*ctx).nbSamples as isize);
    let mut curSampleEnd: size_t = *(*ctx).offsets.offset(0 as ::core::ffi::c_int as isize);
    while grpPtr != grpEnd {
        *(*ctx).dmerAt.offset(*grpPtr as isize) = dmerId;
        if !((*grpPtr as size_t) < curSampleEnd) {
            freq =
                (freq as ::core::ffi::c_uint).wrapping_add(1 as ::core::ffi::c_uint) as U32 as U32;
            if grpPtr.offset(1 as ::core::ffi::c_int as isize) != grpEnd {
                let mut sampleEndPtr: *const size_t =
                    COVER_lower_bound(curOffsetPtr, offsetsEnd, *grpPtr as size_t);
                curSampleEnd = *sampleEndPtr;
                curOffsetPtr = sampleEndPtr.offset(1 as ::core::ffi::c_int as isize);
            }
        }
        grpPtr = grpPtr.offset(1);
    }
    *(*ctx).suffix.offset(dmerId as isize) = freq;
}
unsafe extern "C" fn COVER_selectSegment(
    mut ctx: *const COVER_ctx_t,
    mut freqs: *mut U32,
    mut activeDmers: *mut COVER_map_t,
    mut begin: U32,
    mut end: U32,
    mut parameters: ZDICT_cover_params_t,
) -> COVER_segment_t {
    let k: U32 = parameters.k as U32;
    let d: U32 = parameters.d as U32;
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
    COVER_map_clear(activeDmers);
    activeSegment.begin = begin;
    activeSegment.end = begin;
    activeSegment.score = 0 as U32;
    while activeSegment.end < end {
        let mut newDmer: U32 = *(*ctx).dmerAt.offset(activeSegment.end as isize);
        let mut newDmerOcc: *mut U32 = COVER_map_at(activeDmers, newDmer);
        if *newDmerOcc == 0 as U32 {
            activeSegment.score = (activeSegment.score as ::core::ffi::c_uint)
                .wrapping_add(*freqs.offset(newDmer as isize) as ::core::ffi::c_uint)
                as U32 as U32;
        }
        activeSegment.end = (activeSegment.end as ::core::ffi::c_uint)
            .wrapping_add(1 as ::core::ffi::c_uint) as U32 as U32;
        *newDmerOcc = (*newDmerOcc as ::core::ffi::c_uint).wrapping_add(1 as ::core::ffi::c_uint)
            as U32 as U32;
        if activeSegment.end.wrapping_sub(activeSegment.begin) == dmersInK.wrapping_add(1 as U32) {
            let mut delDmer: U32 = *(*ctx).dmerAt.offset(activeSegment.begin as isize);
            let mut delDmerOcc: *mut U32 = COVER_map_at(activeDmers, delDmer);
            activeSegment.begin = (activeSegment.begin as ::core::ffi::c_uint)
                .wrapping_add(1 as ::core::ffi::c_uint) as U32
                as U32;
            *delDmerOcc = (*delDmerOcc as ::core::ffi::c_uint)
                .wrapping_sub(1 as ::core::ffi::c_uint) as U32 as U32;
            if *delDmerOcc == 0 as U32 {
                COVER_map_remove(activeDmers, delDmer);
                activeSegment.score = (activeSegment.score as ::core::ffi::c_uint)
                    .wrapping_sub(*freqs.offset(delDmer as isize) as ::core::ffi::c_uint)
                    as U32 as U32;
            }
        }
        if activeSegment.score > bestSegment.score {
            bestSegment = activeSegment;
        }
    }
    let mut newBegin: U32 = bestSegment.end;
    let mut newEnd: U32 = bestSegment.begin;
    let mut pos: U32 = 0;
    pos = bestSegment.begin;
    while pos != bestSegment.end {
        let mut freq: U32 = *freqs.offset(*(*ctx).dmerAt.offset(pos as isize) as isize);
        if freq != 0 as U32 {
            newBegin = if newBegin < pos { newBegin } else { pos };
            newEnd = pos.wrapping_add(1 as U32);
        }
        pos = pos.wrapping_add(1);
    }
    bestSegment.begin = newBegin;
    bestSegment.end = newEnd;
    let mut pos_0: U32 = 0;
    pos_0 = bestSegment.begin;
    while pos_0 != bestSegment.end {
        *freqs.offset(*(*ctx).dmerAt.offset(pos_0 as isize) as isize) = 0 as U32;
        pos_0 = pos_0.wrapping_add(1);
    }
    return bestSegment;
}
unsafe extern "C" fn COVER_checkParameters(
    mut parameters: ZDICT_cover_params_t,
    mut maxDictSize: size_t,
) -> ::core::ffi::c_int {
    if parameters.d == 0 as ::core::ffi::c_uint || parameters.k == 0 as ::core::ffi::c_uint {
        return 0 as ::core::ffi::c_int;
    }
    if parameters.k as size_t > maxDictSize {
        return 0 as ::core::ffi::c_int;
    }
    if parameters.d > parameters.k {
        return 0 as ::core::ffi::c_int;
    }
    if parameters.splitPoint <= 0 as ::core::ffi::c_int as ::core::ffi::c_double
        || parameters.splitPoint > 1 as ::core::ffi::c_int as ::core::ffi::c_double
    {
        return 0 as ::core::ffi::c_int;
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn COVER_ctx_destroy(mut ctx: *mut COVER_ctx_t) {
    if ctx.is_null() {
        return;
    }
    if !(*ctx).suffix.is_null() {
        free((*ctx).suffix as *mut ::core::ffi::c_void);
        (*ctx).suffix = ::core::ptr::null_mut::<U32>();
    }
    if !(*ctx).freqs.is_null() {
        free((*ctx).freqs as *mut ::core::ffi::c_void);
        (*ctx).freqs = ::core::ptr::null_mut::<U32>();
    }
    if !(*ctx).dmerAt.is_null() {
        free((*ctx).dmerAt as *mut ::core::ffi::c_void);
        (*ctx).dmerAt = ::core::ptr::null_mut::<U32>();
    }
    if !(*ctx).offsets.is_null() {
        free((*ctx).offsets as *mut ::core::ffi::c_void);
        (*ctx).offsets = ::core::ptr::null_mut::<size_t>();
    }
}
unsafe extern "C" fn COVER_ctx_init(
    mut ctx: *mut COVER_ctx_t,
    mut samplesBuffer: *const ::core::ffi::c_void,
    mut samplesSizes: *const size_t,
    mut nbSamples: ::core::ffi::c_uint,
    mut d: ::core::ffi::c_uint,
    mut splitPoint: ::core::ffi::c_double,
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
                b"Total number of training samples is %u and is invalid.\0" as *const u8
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
                b"Total number of testing samples is %u and is invalid.\0" as *const u8
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
        ::core::mem::size_of::<COVER_ctx_t>() as size_t,
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
    (*ctx).suffixSize = trainingSamplesSize
        .wrapping_sub(
            (if d as usize > ::core::mem::size_of::<U64>() as usize {
                d as size_t
            } else {
                ::core::mem::size_of::<U64>() as size_t
            }),
        )
        .wrapping_add(1 as size_t);
    (*ctx).suffix = malloc(
        (*ctx)
            .suffixSize
            .wrapping_mul(::core::mem::size_of::<U32>() as size_t),
    ) as *mut U32;
    (*ctx).dmerAt = malloc(
        (*ctx)
            .suffixSize
            .wrapping_mul(::core::mem::size_of::<U32>() as size_t),
    ) as *mut U32;
    (*ctx).offsets = malloc(
        (nbSamples.wrapping_add(1 as ::core::ffi::c_uint) as size_t)
            .wrapping_mul(::core::mem::size_of::<size_t>() as size_t),
    ) as *mut size_t;
    if (*ctx).suffix.is_null() || (*ctx).dmerAt.is_null() || (*ctx).offsets.is_null() {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Failed to allocate scratch buffers\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
        COVER_ctx_destroy(ctx);
        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
    }
    (*ctx).freqs = ::core::ptr::null_mut::<U32>();
    (*ctx).d = d;
    let mut i: U32 = 0;
    *(*ctx).offsets.offset(0 as ::core::ffi::c_int as isize) = 0 as size_t;
    i = 1 as U32;
    while i <= nbSamples as U32 {
        *(*ctx).offsets.offset(i as isize) =
            (*(*ctx).offsets.offset(i.wrapping_sub(1 as U32) as isize))
                .wrapping_add(*samplesSizes.offset(i.wrapping_sub(1 as U32) as isize));
        i = i.wrapping_add(1);
    }
    if g_displayLevel >= 2 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"Constructing partial suffix array\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        fflush(stderr);
    }
    let mut i_0: U32 = 0;
    i_0 = 0 as U32;
    while (i_0 as size_t) < (*ctx).suffixSize {
        *(*ctx).suffix.offset(i_0 as isize) = i_0;
        i_0 = i_0.wrapping_add(1);
    }
    stableSort(ctx);
    if g_displayLevel >= 2 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"Computing frequencies\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        fflush(stderr);
    }
    COVER_groupBy(
        (*ctx).suffix as *const ::core::ffi::c_void,
        (*ctx).suffixSize,
        ::core::mem::size_of::<U32>() as size_t,
        ctx,
        if (*ctx).d <= 8 as ::core::ffi::c_uint {
            Some(
                COVER_cmp8
                    as unsafe extern "C" fn(
                        *mut COVER_ctx_t,
                        *const ::core::ffi::c_void,
                        *const ::core::ffi::c_void,
                    ) -> ::core::ffi::c_int,
            )
        } else {
            Some(
                COVER_cmp
                    as unsafe extern "C" fn(
                        *mut COVER_ctx_t,
                        *const ::core::ffi::c_void,
                        *const ::core::ffi::c_void,
                    ) -> ::core::ffi::c_int,
            )
        },
        Some(
            COVER_group
                as unsafe extern "C" fn(
                    *mut COVER_ctx_t,
                    *const ::core::ffi::c_void,
                    *const ::core::ffi::c_void,
                ) -> (),
        ),
    );
    (*ctx).freqs = (*ctx).suffix;
    (*ctx).suffix = ::core::ptr::null_mut::<U32>();
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_warnOnSmallCorpus(
    mut maxDictSize: size_t,
    mut nbDmers: size_t,
    mut displayLevel: ::core::ffi::c_int,
) {
    let ratio: ::core::ffi::c_double =
        nbDmers as ::core::ffi::c_double / maxDictSize as ::core::ffi::c_double;
    if ratio >= 10 as ::core::ffi::c_int as ::core::ffi::c_double {
        return;
    }
    if displayLevel >= 1 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"WARNING: The maximum dictionary size %u is too large compared to the source size %u! size(source)/size(dictionary) = %f, but it should be >= 10! This may lead to a subpar dictionary! We recommend training on sources at least 10x, and preferably 100x the size of the dictionary! \n\0"
                as *const u8 as *const ::core::ffi::c_char,
            maxDictSize as U32,
            nbDmers as U32,
            ratio,
        );
        fflush(stderr);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_computeEpochs(
    mut maxDictSize: U32,
    mut nbDmers: U32,
    mut k: U32,
    mut passes: U32,
) -> COVER_epoch_info_t {
    let minEpochSize: U32 = k.wrapping_mul(10 as U32);
    let mut epochs: COVER_epoch_info_t = COVER_epoch_info_t { num: 0, size: 0 };
    epochs.num = if 1 as U32 > maxDictSize.wrapping_div(k).wrapping_div(passes) {
        1 as U32
    } else {
        maxDictSize.wrapping_div(k).wrapping_div(passes)
    };
    epochs.size = nbDmers.wrapping_div(epochs.num);
    if epochs.size >= minEpochSize {
        return epochs;
    }
    epochs.size = if minEpochSize < nbDmers {
        minEpochSize
    } else {
        nbDmers
    };
    epochs.num = nbDmers.wrapping_div(epochs.size);
    return epochs;
}
unsafe extern "C" fn COVER_buildDictionary(
    mut ctx: *const COVER_ctx_t,
    mut freqs: *mut U32,
    mut activeDmers: *mut COVER_map_t,
    mut dictBuffer: *mut ::core::ffi::c_void,
    mut dictBufferCapacity: size_t,
    mut parameters: ZDICT_cover_params_t,
) -> size_t {
    let dict: *mut BYTE = dictBuffer as *mut BYTE;
    let mut tail: size_t = dictBufferCapacity;
    let epochs: COVER_epoch_info_t = COVER_computeEpochs(
        dictBufferCapacity as U32,
        (*ctx).suffixSize as U32,
        parameters.k as U32,
        4 as U32,
    ) as COVER_epoch_info_t;
    let maxZeroScoreRun: size_t = (if 10 as U32
        > (if (100 as U32) < epochs.num >> 3 as ::core::ffi::c_int {
            100 as U32
        } else {
            epochs.num >> 3 as ::core::ffi::c_int
        }) {
        10 as U32
    } else if (100 as U32) < epochs.num >> 3 as ::core::ffi::c_int {
        100 as U32
    } else {
        epochs.num >> 3 as ::core::ffi::c_int
    }) as size_t;
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
            COVER_selectSegment(ctx, freqs, activeDmers, epochBegin, epochEnd, parameters);
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_trainFromBuffer_cover(
    mut dictBuffer: *mut ::core::ffi::c_void,
    mut dictBufferCapacity: size_t,
    mut samplesBuffer: *const ::core::ffi::c_void,
    mut samplesSizes: *const size_t,
    mut nbSamples: ::core::ffi::c_uint,
    mut parameters: ZDICT_cover_params_t,
) -> size_t {
    let dict: *mut BYTE = dictBuffer as *mut BYTE;
    let mut ctx: COVER_ctx_t = COVER_ctx_t {
        samples: ::core::ptr::null::<BYTE>(),
        offsets: ::core::ptr::null_mut::<size_t>(),
        samplesSizes: ::core::ptr::null::<size_t>(),
        nbSamples: 0,
        nbTrainSamples: 0,
        nbTestSamples: 0,
        suffix: ::core::ptr::null_mut::<U32>(),
        suffixSize: 0,
        freqs: ::core::ptr::null_mut::<U32>(),
        dmerAt: ::core::ptr::null_mut::<U32>(),
        d: 0,
    };
    let mut activeDmers: COVER_map_t = COVER_map_t {
        data: ::core::ptr::null_mut::<COVER_map_pair_t>(),
        sizeLog: 0,
        size: 0,
        sizeMask: 0,
    };
    parameters.splitPoint = 1.0f64;
    g_displayLevel = parameters.zParams.notificationLevel as ::core::ffi::c_int;
    if COVER_checkParameters(parameters, dictBufferCapacity) == 0 {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Cover parameters incorrect\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
        return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
    }
    if nbSamples == 0 as ::core::ffi::c_uint {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Cover must have at least one input file\n\0" as *const u8
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
    let initVal: size_t = COVER_ctx_init(
        &raw mut ctx,
        samplesBuffer,
        samplesSizes,
        nbSamples,
        parameters.d,
        parameters.splitPoint,
    ) as size_t;
    if ERR_isError(initVal) != 0 {
        return initVal;
    }
    COVER_warnOnSmallCorpus(dictBufferCapacity, ctx.suffixSize, g_displayLevel);
    if COVER_map_init(
        &raw mut activeDmers,
        (parameters.k as U32)
            .wrapping_sub(parameters.d as U32)
            .wrapping_add(1 as U32),
    ) == 0
    {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Failed to allocate dmer map: out of memory\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
        COVER_ctx_destroy(&raw mut ctx);
        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
    }
    if g_displayLevel >= 2 as ::core::ffi::c_int {
        fprintf(
            stderr,
            b"Building dictionary\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        fflush(stderr);
    }
    let tail: size_t = COVER_buildDictionary(
        &raw mut ctx,
        ctx.freqs,
        &raw mut activeDmers,
        dictBuffer,
        dictBufferCapacity,
        parameters,
    ) as size_t;
    let dictionarySize: size_t = ZDICT_finalizeDictionary(
        dict as *mut ::core::ffi::c_void,
        dictBufferCapacity,
        dict.offset(tail as isize) as *const ::core::ffi::c_void,
        dictBufferCapacity.wrapping_sub(tail),
        samplesBuffer,
        samplesSizes,
        nbSamples,
        parameters.zParams,
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
    COVER_ctx_destroy(&raw mut ctx);
    COVER_map_destroy(&raw mut activeDmers);
    return dictionarySize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_checkTotalCompressedSize(
    parameters: ZDICT_cover_params_t,
    mut samplesSizes: *const size_t,
    mut samples: *const BYTE,
    mut offsets: *mut size_t,
    mut nbTrainSamples: size_t,
    mut nbSamples: size_t,
    dict: *mut BYTE,
    mut dictBufferCapacity: size_t,
) -> size_t {
    let mut totalCompressedSize: size_t = -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    let mut cctx: *mut ZSTD_CCtx = ::core::ptr::null_mut::<ZSTD_CCtx>();
    let mut cdict: *mut ZSTD_CDict = ::core::ptr::null_mut::<ZSTD_CDict>();
    let mut dst: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
    let mut dstCapacity: size_t = 0;
    let mut i: size_t = 0;
    let mut maxSampleSize: size_t = 0 as size_t;
    i = if parameters.splitPoint < 1.0f64 {
        nbTrainSamples
    } else {
        0 as size_t
    };
    while i < nbSamples {
        maxSampleSize = if *samplesSizes.offset(i as isize) > maxSampleSize {
            *samplesSizes.offset(i as isize)
        } else {
            maxSampleSize
        };
        i = i.wrapping_add(1);
    }
    dstCapacity = ZSTD_compressBound(maxSampleSize);
    dst = malloc(dstCapacity);
    cctx = ZSTD_createCCtx();
    cdict = ZSTD_createCDict(
        dict as *const ::core::ffi::c_void,
        dictBufferCapacity,
        parameters.zParams.compressionLevel,
    );
    if !(dst.is_null() || cctx.is_null() || cdict.is_null()) {
        totalCompressedSize = dictBufferCapacity;
        i = if parameters.splitPoint < 1.0f64 {
            nbTrainSamples
        } else {
            0 as size_t
        };
        while i < nbSamples {
            let size: size_t = ZSTD_compress_usingCDict(
                cctx,
                dst,
                dstCapacity,
                samples.offset(*offsets.offset(i as isize) as isize) as *const ::core::ffi::c_void,
                *samplesSizes.offset(i as isize),
                cdict,
            ) as size_t;
            if ERR_isError(size) != 0 {
                totalCompressedSize = size;
                break;
            } else {
                totalCompressedSize = (totalCompressedSize as ::core::ffi::c_ulong)
                    .wrapping_add(size as ::core::ffi::c_ulong)
                    as size_t as size_t;
                i = i.wrapping_add(1);
            }
        }
    }
    ZSTD_freeCCtx(cctx);
    ZSTD_freeCDict(cdict);
    if !dst.is_null() {
        free(dst);
    }
    return totalCompressedSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_init(mut best: *mut COVER_best_t) {
    if best.is_null() {
        return;
    }
    &raw mut (*best).mutex;
    &raw mut (*best).cond;
    (*best).liveJobs = 0 as size_t;
    (*best).dict = NULL;
    (*best).dictSize = 0 as size_t;
    (*best).compressedSize = -(1 as ::core::ffi::c_int) as size_t;
    memset(
        &raw mut (*best).parameters as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<ZDICT_cover_params_t>() as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_wait(mut best: *mut COVER_best_t) {
    if best.is_null() {
        return;
    }
    &raw mut (*best).mutex;
    while (*best).liveJobs != 0 as size_t {
        &raw mut (*best).cond;
        &raw mut (*best).mutex;
    }
    &raw mut (*best).mutex;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_destroy(mut best: *mut COVER_best_t) {
    if best.is_null() {
        return;
    }
    COVER_best_wait(best);
    if !(*best).dict.is_null() {
        free((*best).dict);
    }
    &raw mut (*best).mutex;
    &raw mut (*best).cond;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_start(mut best: *mut COVER_best_t) {
    if best.is_null() {
        return;
    }
    &raw mut (*best).mutex;
    (*best).liveJobs = (*best).liveJobs.wrapping_add(1);
    &raw mut (*best).mutex;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_best_finish(
    mut best: *mut COVER_best_t,
    mut parameters: ZDICT_cover_params_t,
    mut selection: COVER_dictSelection_t,
) {
    let mut dict: *mut ::core::ffi::c_void = selection.dictContent as *mut ::core::ffi::c_void;
    let mut compressedSize: size_t = selection.totalCompressedSize;
    let mut dictSize: size_t = selection.dictSize;
    if best.is_null() {
        return;
    }
    let mut liveJobs: size_t = 0;
    &raw mut (*best).mutex;
    (*best).liveJobs = (*best).liveJobs.wrapping_sub(1);
    liveJobs = (*best).liveJobs;
    if compressedSize < (*best).compressedSize {
        if (*best).dict.is_null() || (*best).dictSize < dictSize {
            if !(*best).dict.is_null() {
                free((*best).dict);
            }
            (*best).dict = malloc(dictSize);
            if (*best).dict.is_null() {
                (*best).compressedSize = -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
                (*best).dictSize = 0 as size_t;
                &raw mut (*best).cond;
                &raw mut (*best).mutex;
                return;
            }
        }
        if !dict.is_null() {
            memcpy((*best).dict, dict, dictSize);
            (*best).dictSize = dictSize;
            (*best).parameters = parameters;
            (*best).compressedSize = compressedSize;
        }
    }
    if liveJobs == 0 as size_t {
        &raw mut (*best).cond;
    }
    &raw mut (*best).mutex;
}
unsafe extern "C" fn setDictSelection(
    mut buf: *mut BYTE,
    mut s: size_t,
    mut csz: size_t,
) -> COVER_dictSelection_t {
    let mut ds: COVER_dictSelection_t = COVER_dictSelection_t {
        dictContent: ::core::ptr::null_mut::<BYTE>(),
        dictSize: 0,
        totalCompressedSize: 0,
    };
    ds.dictContent = buf;
    ds.dictSize = s;
    ds.totalCompressedSize = csz;
    return ds;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_dictSelectionError(mut error: size_t) -> COVER_dictSelection_t {
    return setDictSelection(::core::ptr::null_mut::<BYTE>(), 0 as size_t, error);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_dictSelectionIsError(
    mut selection: COVER_dictSelection_t,
) -> ::core::ffi::c_uint {
    return (ERR_isError(selection.totalCompressedSize) != 0 || selection.dictContent.is_null())
        as ::core::ffi::c_int as ::core::ffi::c_uint;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_dictSelectionFree(mut selection: COVER_dictSelection_t) {
    free(selection.dictContent as *mut ::core::ffi::c_void);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn COVER_selectDict(
    mut customDictContent: *mut BYTE,
    mut dictBufferCapacity: size_t,
    mut dictContentSize: size_t,
    mut samplesBuffer: *const BYTE,
    mut samplesSizes: *const size_t,
    mut nbFinalizeSamples: ::core::ffi::c_uint,
    mut nbCheckSamples: size_t,
    mut nbSamples: size_t,
    mut params: ZDICT_cover_params_t,
    mut offsets: *mut size_t,
    mut totalCompressedSize: size_t,
) -> COVER_dictSelection_t {
    let mut largestDict: size_t = 0 as size_t;
    let mut largestCompressed: size_t = 0 as size_t;
    let mut customDictContentEnd: *mut BYTE = customDictContent.offset(dictContentSize as isize);
    let mut largestDictbuffer: *mut BYTE = malloc(dictBufferCapacity) as *mut BYTE;
    let mut candidateDictBuffer: *mut BYTE = malloc(dictBufferCapacity) as *mut BYTE;
    let mut regressionTolerance: ::core::ffi::c_double =
        params.shrinkDictMaxRegression as ::core::ffi::c_double / 100.0f64 + 1.00f64;
    if largestDictbuffer.is_null() || candidateDictBuffer.is_null() {
        free(largestDictbuffer as *mut ::core::ffi::c_void);
        free(candidateDictBuffer as *mut ::core::ffi::c_void);
        return COVER_dictSelectionError(dictContentSize);
    }
    memcpy(
        largestDictbuffer as *mut ::core::ffi::c_void,
        customDictContent as *const ::core::ffi::c_void,
        dictContentSize,
    );
    dictContentSize = ZDICT_finalizeDictionary(
        largestDictbuffer as *mut ::core::ffi::c_void,
        dictBufferCapacity,
        customDictContent as *const ::core::ffi::c_void,
        dictContentSize,
        samplesBuffer as *const ::core::ffi::c_void,
        samplesSizes,
        nbFinalizeSamples,
        params.zParams,
    );
    if ZDICT_isError(dictContentSize) != 0 {
        free(largestDictbuffer as *mut ::core::ffi::c_void);
        free(candidateDictBuffer as *mut ::core::ffi::c_void);
        return COVER_dictSelectionError(dictContentSize);
    }
    totalCompressedSize = COVER_checkTotalCompressedSize(
        params,
        samplesSizes,
        samplesBuffer,
        offsets,
        nbCheckSamples,
        nbSamples,
        largestDictbuffer,
        dictContentSize,
    );
    if ERR_isError(totalCompressedSize) != 0 {
        free(largestDictbuffer as *mut ::core::ffi::c_void);
        free(candidateDictBuffer as *mut ::core::ffi::c_void);
        return COVER_dictSelectionError(totalCompressedSize);
    }
    if params.shrinkDict == 0 as ::core::ffi::c_uint {
        free(candidateDictBuffer as *mut ::core::ffi::c_void);
        return setDictSelection(largestDictbuffer, dictContentSize, totalCompressedSize);
    }
    largestDict = dictContentSize;
    largestCompressed = totalCompressedSize;
    dictContentSize = ZDICT_DICTSIZE_MIN as size_t;
    while dictContentSize < largestDict {
        memcpy(
            candidateDictBuffer as *mut ::core::ffi::c_void,
            largestDictbuffer as *const ::core::ffi::c_void,
            largestDict,
        );
        dictContentSize = ZDICT_finalizeDictionary(
            candidateDictBuffer as *mut ::core::ffi::c_void,
            dictBufferCapacity,
            customDictContentEnd.offset(-(dictContentSize as isize)) as *const ::core::ffi::c_void,
            dictContentSize,
            samplesBuffer as *const ::core::ffi::c_void,
            samplesSizes,
            nbFinalizeSamples,
            params.zParams,
        );
        if ZDICT_isError(dictContentSize) != 0 {
            free(largestDictbuffer as *mut ::core::ffi::c_void);
            free(candidateDictBuffer as *mut ::core::ffi::c_void);
            return COVER_dictSelectionError(dictContentSize);
        }
        totalCompressedSize = COVER_checkTotalCompressedSize(
            params,
            samplesSizes,
            samplesBuffer,
            offsets,
            nbCheckSamples,
            nbSamples,
            candidateDictBuffer,
            dictContentSize,
        );
        if ERR_isError(totalCompressedSize) != 0 {
            free(largestDictbuffer as *mut ::core::ffi::c_void);
            free(candidateDictBuffer as *mut ::core::ffi::c_void);
            return COVER_dictSelectionError(totalCompressedSize);
        }
        if totalCompressedSize as ::core::ffi::c_double
            <= largestCompressed as ::core::ffi::c_double * regressionTolerance
        {
            free(largestDictbuffer as *mut ::core::ffi::c_void);
            return setDictSelection(candidateDictBuffer, dictContentSize, totalCompressedSize);
        }
        dictContentSize = (dictContentSize as ::core::ffi::c_ulong)
            .wrapping_mul(2 as ::core::ffi::c_ulong) as size_t as size_t;
    }
    dictContentSize = largestDict;
    totalCompressedSize = largestCompressed;
    free(candidateDictBuffer as *mut ::core::ffi::c_void);
    return setDictSelection(largestDictbuffer, dictContentSize, totalCompressedSize);
}
unsafe extern "C" fn COVER_tryParameters(mut opaque: *mut ::core::ffi::c_void) {
    let data: *mut COVER_tryParameters_data_t = opaque as *mut COVER_tryParameters_data_t;
    let ctx: *const COVER_ctx_t = (*data).ctx;
    let parameters: ZDICT_cover_params_t = (*data).parameters;
    let mut dictBufferCapacity: size_t = (*data).dictBufferCapacity;
    let mut totalCompressedSize: size_t = -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    let mut activeDmers: COVER_map_t = COVER_map_t {
        data: ::core::ptr::null_mut::<COVER_map_pair_t>(),
        sizeLog: 0,
        size: 0,
        sizeMask: 0,
    };
    let dict: *mut BYTE = malloc(dictBufferCapacity) as *mut BYTE;
    let mut selection: COVER_dictSelection_t =
        COVER_dictSelectionError(-(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t);
    let freqs: *mut U32 = malloc(
        (*ctx)
            .suffixSize
            .wrapping_mul(::core::mem::size_of::<U32>() as size_t),
    ) as *mut U32;
    if COVER_map_init(
        &raw mut activeDmers,
        (parameters.k as U32)
            .wrapping_sub(parameters.d as U32)
            .wrapping_add(1 as U32),
    ) == 0
    {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Failed to allocate dmer map: out of memory\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
    } else if dict.is_null() || freqs.is_null() {
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
            (*ctx)
                .suffixSize
                .wrapping_mul(::core::mem::size_of::<U32>() as size_t),
        );
        let tail: size_t = COVER_buildDictionary(
            ctx,
            freqs,
            &raw mut activeDmers,
            dict as *mut ::core::ffi::c_void,
            dictBufferCapacity,
            parameters,
        ) as size_t;
        selection = COVER_selectDict(
            dict.offset(tail as isize),
            dictBufferCapacity,
            dictBufferCapacity.wrapping_sub(tail),
            (*ctx).samples,
            (*ctx).samplesSizes,
            (*ctx).nbTrainSamples as ::core::ffi::c_uint,
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
    COVER_map_destroy(&raw mut activeDmers);
    COVER_dictSelectionFree(selection);
    free(freqs as *mut ::core::ffi::c_void);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZDICT_optimizeTrainFromBuffer_cover(
    mut dictBuffer: *mut ::core::ffi::c_void,
    mut dictBufferCapacity: size_t,
    mut samplesBuffer: *const ::core::ffi::c_void,
    mut samplesSizes: *const size_t,
    mut nbSamples: ::core::ffi::c_uint,
    mut parameters: *mut ZDICT_cover_params_t,
) -> size_t {
    let nbThreads: ::core::ffi::c_uint = (*parameters).nbThreads;
    let splitPoint: ::core::ffi::c_double = if (*parameters).splitPoint <= 0.0f64 {
        COVER_DEFAULT_SPLITPOINT
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
                b"Incorrect parameters\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
        return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
    }
    if kMinK < kMaxD || kMaxK < kMinK {
        if displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Incorrect parameters\n\0" as *const u8 as *const ::core::ffi::c_char,
            );
            fflush(stderr);
        }
        return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
    }
    if nbSamples == 0 as ::core::ffi::c_uint {
        if g_displayLevel >= 1 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Cover must have at least one input file\n\0" as *const u8
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
    if nbThreads > 1 as ::core::ffi::c_uint {
        pool = POOL_create(nbThreads as size_t, 1 as size_t);
        if pool.is_null() {
            return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
        }
    }
    COVER_best_init(&raw mut best);
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
        let mut ctx: COVER_ctx_t = COVER_ctx_t {
            samples: ::core::ptr::null::<BYTE>(),
            offsets: ::core::ptr::null_mut::<size_t>(),
            samplesSizes: ::core::ptr::null::<size_t>(),
            nbSamples: 0,
            nbTrainSamples: 0,
            nbTestSamples: 0,
            suffix: ::core::ptr::null_mut::<U32>(),
            suffixSize: 0,
            freqs: ::core::ptr::null_mut::<U32>(),
            dmerAt: ::core::ptr::null_mut::<U32>(),
            d: 0,
        };
        if displayLevel >= 3 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"d=%u\n\0" as *const u8 as *const ::core::ffi::c_char,
                d,
            );
            fflush(stderr);
        }
        let initVal: size_t = COVER_ctx_init(
            &raw mut ctx,
            samplesBuffer,
            samplesSizes,
            nbSamples,
            d,
            splitPoint,
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
            COVER_warnOnSmallCorpus(dictBufferCapacity, ctx.suffixSize, displayLevel);
            warned = 1 as ::core::ffi::c_int;
        }
        k = kMinK;
        while k <= kMaxK {
            let mut data: *mut COVER_tryParameters_data_t =
                malloc(::core::mem::size_of::<COVER_tryParameters_data_t>() as size_t)
                    as *mut COVER_tryParameters_data_t;
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
                COVER_ctx_destroy(&raw mut ctx);
                POOL_free(pool);
                return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
            }
            (*data).ctx = &raw mut ctx;
            (*data).best = &raw mut best;
            (*data).dictBufferCapacity = dictBufferCapacity;
            (*data).parameters = *parameters;
            (*data).parameters.k = k;
            (*data).parameters.d = d;
            (*data).parameters.splitPoint = splitPoint;
            (*data).parameters.steps = kSteps;
            (*data).parameters.shrinkDict = shrinkDict;
            (*data).parameters.zParams.notificationLevel = g_displayLevel as ::core::ffi::c_uint;
            if COVER_checkParameters((*data).parameters, dictBufferCapacity) == 0 {
                if g_displayLevel >= 1 as ::core::ffi::c_int {
                    fprintf(
                        stderr,
                        b"Cover parameters incorrect\n\0" as *const u8
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
                            COVER_tryParameters
                                as unsafe extern "C" fn(*mut ::core::ffi::c_void) -> (),
                        ),
                        data as *mut ::core::ffi::c_void,
                    );
                } else {
                    COVER_tryParameters(data as *mut ::core::ffi::c_void);
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
        COVER_ctx_destroy(&raw mut ctx);
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
    *parameters = best.parameters;
    memcpy(dictBuffer, best.dict, dictSize);
    COVER_best_destroy(&raw mut best);
    POOL_free(pool);
    return dictSize;
}
