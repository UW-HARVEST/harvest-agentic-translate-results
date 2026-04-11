extern "C" {
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn memmove(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn memset(
        __s: *mut libc::c_void,
        __c: libc::c_int,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn memcmp(
        __s1: *const libc::c_void,
        __s2: *const libc::c_void,
        __n: size_t,
    ) -> libc::c_int;
    fn strcmp(
        __s1: *const libc::c_char,
        __s2: *const libc::c_char,
    ) -> libc::c_int;
    fn strlen(__s: *const libc::c_char) -> size_t;
    fn realloc(__ptr: *mut libc::c_void, __size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn __assert_fail(
        __assertion: *const libc::c_char,
        __file: *const libc::c_char,
        __line: libc::c_uint,
        __function: *const libc::c_char,
    ) -> !;
    fn sprintf(
        __s: *mut libc::c_char,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stbds_array_header {
    pub length: size_t,
    pub capacity: size_t,
    pub hash_table: *mut libc::c_void,
    pub temp: ptrdiff_t,
}
pub type ptrdiff_t = isize;
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stbds_string_arena {
    pub storage: *mut stbds_string_block,
    pub remaining: size_t,
    pub block: libc::c_uchar,
    pub mode: libc::c_uchar,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stbds_string_block {
    pub next: *mut stbds_string_block,
    pub storage: [libc::c_char; 8],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stbds_hash_index {
    pub temp_key: *mut libc::c_char,
    pub slot_count: size_t,
    pub used_count: size_t,
    pub used_count_threshold: size_t,
    pub used_count_shrink_threshold: size_t,
    pub tombstone_count: size_t,
    pub tombstone_count_threshold: size_t,
    pub seed: size_t,
    pub slot_count_log2: size_t,
    pub string: stbds_string_arena,
    pub storage: *mut stbds_hash_bucket,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stbds_hash_bucket {
    pub hash: [size_t; 8],
    pub index: [ptrdiff_t; 8],
}
pub const STBDS_SH_STRDUP: C2RustUnnamed = 2;
pub const STBDS_SH_DEFAULT: C2RustUnnamed = 1;
pub const STBDS_SH_ARENA: C2RustUnnamed = 3;
pub type C2RustUnnamed = libc::c_uint;
pub const STBDS_SH_NONE: C2RustUnnamed = 0;
pub const NULL_0: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const STBDS_HM_STRING: libc::c_int = 1 as libc::c_int;
pub const __ASSERT_FUNCTION: [libc::c_char; 19] = unsafe {
    std::mem::transmute::<[u8; 19], [libc::c_char; 19]>(*b"void arr_push(int)\0")
};
#[no_mangle]
pub unsafe extern "C" fn stbds_arrgrowf(
    mut a: *mut libc::c_void,
    mut elemsize: size_t,
    mut addlen: size_t,
    mut min_cap: size_t,
) -> *mut libc::c_void {
    let mut temp: stbds_array_header = stbds_array_header {
        length: 0 as size_t,
        capacity: 0,
        hash_table: std::ptr::null_mut::<libc::c_void>(),
        temp: 0,
    };
    let mut b: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
    let mut min_len: size_t = ((if !a.is_null() {
        (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).length
            as ptrdiff_t
    } else {
        0 as ptrdiff_t
    }) as size_t)
        .wrapping_add(addlen);
    if min_len > min_cap {
        min_cap = min_len;
    }
    if min_cap
        <= (if !a.is_null() {
            (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).capacity
        } else {
            0 as size_t
        })
    {
        return a;
    }
    if min_cap
        < (2 as size_t).wrapping_mul(
            (if !a.is_null() {
                (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize)))
                    .capacity
            } else {
                0 as size_t
            }),
        )
    {
        min_cap = (2 as size_t).wrapping_mul(
            (if !a.is_null() {
                (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize)))
                    .capacity
            } else {
                0 as size_t
            }),
        );
    } else if min_cap < 4 as size_t {
        min_cap = 4 as size_t;
    }
    b = realloc(
        (if !a.is_null() {
            (a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))
        } else {
            std::ptr::null_mut::<stbds_array_header>()
        }) as *mut libc::c_void,
        elemsize
            .wrapping_mul(min_cap)
            .wrapping_add(std::mem::size_of::<stbds_array_header>() as size_t),
    );
    b = (b as *mut libc::c_char)
        .offset(std::mem::size_of::<stbds_array_header>() as usize as isize)
        as *mut libc::c_void;
    if a.is_null() {
        (*(b as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).length =
            0 as size_t;
        let ref mut fresh2 = (*(b as *mut stbds_array_header)
            .offset(-(1 as libc::c_int as isize)))
        .hash_table;
        *fresh2 = std::ptr::null_mut::<libc::c_void>();
        (*(b as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).temp =
            0 as ptrdiff_t;
    }
    (*(b as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).capacity =
        min_cap;
    return b;
}
#[no_mangle]
pub unsafe extern "C" fn stbds_arrfreef(mut a: *mut libc::c_void) {
    free(
        (a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))
            as *mut libc::c_void,
    );
}
pub const STBDS_BUCKET_LENGTH: libc::c_int = 8 as libc::c_int;
pub const STBDS_BUCKET_MASK: libc::c_int = STBDS_BUCKET_LENGTH - 1 as libc::c_int;
pub const STBDS_INDEX_EMPTY: libc::c_int = -(1 as libc::c_int);
pub const STBDS_INDEX_DELETED: libc::c_int = -(2 as libc::c_int);
pub const STBDS_HASH_EMPTY: libc::c_int = 0 as libc::c_int;
pub const STBDS_HASH_DELETED: libc::c_int = 1 as libc::c_int;
static mut stbds_hash_seed: size_t = 0x31415926 as libc::c_int as size_t;
#[no_mangle]
pub unsafe extern "C" fn stbds_rand_seed(mut seed: size_t) {
    stbds_hash_seed = seed;
}
pub const STBDS_SIZE_T_BITS: usize =
    (std::mem::size_of::<size_t>() as usize).wrapping_mul(8 as usize);
 extern "C" fn stbds_probe_position(
    mut hash: size_t,
    mut slot_count: size_t,
    mut slot_log2: size_t,
) -> size_t {
    let mut pos: size_t = 0;
    pos = hash & slot_count.wrapping_sub(1 as size_t);
    return pos;
}
 extern "C" fn stbds_log2(mut slot_count: size_t) -> size_t {
    let mut n: size_t = 0 as size_t;
    while slot_count > 1 as size_t {
        slot_count >>= 1 as libc::c_int;
        n = n.wrapping_add(1);
    }
    return n;
}
unsafe extern "C" fn stbds_make_hash_index(
    mut slot_count: size_t,
    mut ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let mut t: *mut stbds_hash_index = std::ptr::null_mut::<stbds_hash_index>();
    t = realloc(
        std::ptr::null_mut::<libc::c_void>(),
        (slot_count
            >> (if 8 as libc::c_int == 8 as libc::c_int {
                3 as libc::c_int
            } else {
                2 as libc::c_int
            }))
        .wrapping_mul(std::mem::size_of::<stbds_hash_bucket>() as size_t)
        .wrapping_add(std::mem::size_of::<stbds_hash_index>() as size_t)
        .wrapping_add(64 as size_t)
        .wrapping_sub(1 as size_t),
    ) as *mut stbds_hash_index;
    (*t).storage = ((t.offset(1 as libc::c_int as isize) as size_t)
        .wrapping_add(64 as size_t)
        .wrapping_sub(1 as size_t)
        & !(64 as libc::c_int - 1 as libc::c_int) as size_t)
        as *mut stbds_hash_bucket;
    (*t).slot_count = slot_count;
    (*t).slot_count_log2 = stbds_log2(slot_count);
    (*t).tombstone_count = 0 as size_t;
    (*t).used_count = 0 as size_t;
    (*t).used_count_threshold = slot_count.wrapping_sub(slot_count >> 2 as libc::c_int);
    (*t).tombstone_count_threshold =
        (slot_count >> 3 as libc::c_int).wrapping_add(slot_count >> 4 as libc::c_int);
    (*t).used_count_shrink_threshold = slot_count >> 2 as libc::c_int;
    if slot_count <= STBDS_BUCKET_LENGTH as size_t {
        (*t).used_count_shrink_threshold = 0 as size_t;
    }
    '_c2rust_label: {
        if (*t)
            .used_count_threshold
            .wrapping_add((*t).tombstone_count_threshold)
            < (*t).slot_count
        {
        } else {
            __assert_fail(
                b"t->used_count_threshold + t->tombstone_count_threshold < t->slot_count\0"
                    as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-LWI3Rj/driver/c_src/src/lib.c\0" as *const u8
                    as *const libc::c_char,
                401 as libc::c_uint,
                b"stbds_hash_index *stbds_make_hash_index(size_t, stbds_hash_index *)\0"
                    as *const u8 as *const libc::c_char,
            );
        }
    };
    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        let mut a: size_t = 0;
        let mut b: size_t = 0;
        let mut temp: size_t = 0;
        memset(
            &raw mut (*t).string as *mut libc::c_void,
            0 as libc::c_int,
            std::mem::size_of::<stbds_string_arena>() as size_t,
        );
        (*t).seed = stbds_hash_seed;
        temp = (0x87b0b0fd as libc::c_uint
            ^ 2147001325 as libc::c_int as libc::c_uint) as size_t;
        temp <<= 16 as libc::c_int;
        temp <<= 16 as libc::c_int;
        temp >>= 16 as libc::c_int;
        temp >>= 16 as libc::c_int;
        a = 0x27bb2ee6 as libc::c_int as size_t;
        a <<= 16 as libc::c_int;
        a <<= 16 as libc::c_int;
        a = (a as libc::c_ulong
            ^ (temp ^ 2147001325 as libc::c_int as size_t) as libc::c_ulong)
            as size_t;
        temp = (0xb504f32d as libc::c_uint
            ^ 715136305 as libc::c_int as libc::c_uint) as size_t;
        temp <<= 16 as libc::c_int;
        temp <<= 16 as libc::c_int;
        temp >>= 16 as libc::c_int;
        temp >>= 16 as libc::c_int;
        b = 0 as size_t;
        b <<= 16 as libc::c_int;
        b <<= 16 as libc::c_int;
        b = (b as libc::c_ulong
            ^ (temp ^ 715136305 as libc::c_int as size_t) as libc::c_ulong)
            as size_t;
        stbds_hash_seed = stbds_hash_seed.wrapping_mul(a).wrapping_add(b);
    }
    let mut i: size_t = 0;
    let mut j: size_t = 0;
    i = 0 as size_t;
    while i < slot_count
        >> (if STBDS_BUCKET_LENGTH == 8 as libc::c_int {
            3 as libc::c_int
        } else {
            2 as libc::c_int
        })
    {
        let mut b_0: *mut stbds_hash_bucket =
            (*t).storage.offset(i as isize) as *mut stbds_hash_bucket;
        j = 0 as size_t;
        while j < STBDS_BUCKET_LENGTH as size_t {
            (*b_0).hash[j as usize] = STBDS_HASH_EMPTY as size_t;
            j = j.wrapping_add(1);
        }
        j = 0 as size_t;
        while j < STBDS_BUCKET_LENGTH as size_t {
            (*b_0).index[j as usize] = STBDS_INDEX_EMPTY as ptrdiff_t;
            j = j.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    if !ot.is_null() {
        let mut i_0: size_t = 0;
        let mut j_0: size_t = 0;
        (*t).used_count = (*ot).used_count;
        i_0 = 0 as size_t;
        while i_0
            < (*ot).slot_count
                >> (if STBDS_BUCKET_LENGTH == 8 as libc::c_int {
                    3 as libc::c_int
                } else {
                    2 as libc::c_int
                })
        {
            let mut ob: *mut stbds_hash_bucket =
                (*ot).storage.offset(i_0 as isize) as *mut stbds_hash_bucket;
            j_0 = 0 as size_t;
            while j_0 < STBDS_BUCKET_LENGTH as size_t {
                if (*ob).index[j_0 as usize] >= 0 as ptrdiff_t {
                    let mut hash: size_t = (*ob).hash[j_0 as usize];
                    let mut pos: size_t =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step: size_t = STBDS_BUCKET_LENGTH as size_t;
                    's_177: loop {
                        let mut limit: size_t = 0;
                        let mut z: size_t = 0;
                        let mut bucket: *mut stbds_hash_bucket =
                            std::ptr::null_mut::<stbds_hash_bucket>();
                        bucket = (*t).storage.offset(
                            (pos >> (if STBDS_BUCKET_LENGTH == 8 as libc::c_int {
                                3 as libc::c_int
                            } else {
                                2 as libc::c_int
                            })) as isize,
                        ) as *mut stbds_hash_bucket;
                        z = pos & STBDS_BUCKET_MASK as size_t;
                        while z < STBDS_BUCKET_LENGTH as size_t {
                            if (*bucket).hash[z as usize] == 0 as size_t {
                                (*bucket).hash[z as usize] = hash;
                                (*bucket).index[z as usize] = (*ob).index[j_0 as usize];
                                break 's_177;
                            } else {
                                z = z.wrapping_add(1);
                            }
                        }
                        limit = pos & STBDS_BUCKET_MASK as size_t;
                        z = 0 as size_t;
                        while z < limit {
                            if (*bucket).hash[z as usize] == 0 as size_t {
                                (*bucket).hash[z as usize] = hash;
                                (*bucket).index[z as usize] = (*ob).index[j_0 as usize];
                                break 's_177;
                            } else {
                                z = z.wrapping_add(1);
                            }
                        }
                        pos = (pos as libc::c_ulong)
                            .wrapping_add(step as libc::c_ulong)
                            as size_t as size_t;
                        step = (step as libc::c_ulong)
                            .wrapping_add(STBDS_BUCKET_LENGTH as libc::c_ulong)
                            as size_t as size_t;
                        pos = (pos as libc::c_ulong
                            & (*t).slot_count.wrapping_sub(1 as size_t) as libc::c_ulong)
                            as size_t;
                    }
                }
                j_0 = j_0.wrapping_add(1);
            }
            i_0 = i_0.wrapping_add(1);
        }
    }
    return t;
}
#[no_mangle]
pub unsafe extern "C" fn stbds_hash_string(
    mut str: *mut libc::c_char,
    mut seed: size_t,
) -> size_t {
    let mut hash: size_t = seed;
    while *str != 0 {
        let fresh3 = str;
        str = str.offset(1);
        hash = (hash << 9 as libc::c_int
            | hash >> STBDS_SIZE_T_BITS.wrapping_sub(9 as usize))
        .wrapping_add(*fresh3 as libc::c_uchar as size_t);
    }
    hash = (hash as libc::c_ulong ^ seed as libc::c_ulong) as size_t;
    hash = (!hash).wrapping_add(hash << 18 as libc::c_int);
    hash = (hash as libc::c_ulong
        ^ (hash
            ^ (hash >> 31 as libc::c_int
                | hash << STBDS_SIZE_T_BITS.wrapping_sub(31 as usize)))
            as libc::c_ulong) as size_t;
    hash = hash.wrapping_mul(21 as size_t);
    hash = (hash as libc::c_ulong
        ^ (hash
            ^ (hash >> 11 as libc::c_int
                | hash << STBDS_SIZE_T_BITS.wrapping_sub(11 as usize)))
            as libc::c_ulong) as size_t;
    hash = (hash as libc::c_ulong)
        .wrapping_add((hash << 6 as libc::c_int) as libc::c_ulong)
        as size_t as size_t;
    hash = (hash as libc::c_ulong
        ^ (hash >> 22 as libc::c_int | hash << STBDS_SIZE_T_BITS.wrapping_sub(22 as usize))
            as libc::c_ulong) as size_t;
    return hash.wrapping_add(seed);
}
pub const STBDS_SIPHASH_C_ROUNDS: libc::c_int = 2 as libc::c_int;
pub const STBDS_SIPHASH_D_ROUNDS: libc::c_int = 4 as libc::c_int;
unsafe extern "C" fn stbds_siphash_bytes(
    mut p: *mut libc::c_void,
    mut len: size_t,
    mut seed: size_t,
) -> size_t {
    let mut d: *mut libc::c_uchar = p as *mut libc::c_uchar;
    let mut i: size_t = 0;
    let mut j: size_t = 0;
    let mut v0: size_t = 0;
    let mut v1: size_t = 0;
    let mut v2: size_t = 0;
    let mut v3: size_t = 0;
    let mut data: size_t = 0;
    v0 = (((0x736f6d65 as libc::c_int as size_t) << 16 as libc::c_int)
        << 16 as libc::c_int)
        .wrapping_add(0x70736575 as libc::c_int as size_t)
        ^ seed;
    v1 = (((0x646f7261 as libc::c_int as size_t) << 16 as libc::c_int)
        << 16 as libc::c_int)
        .wrapping_add(0x6e646f6d as libc::c_int as size_t)
        ^ !seed;
    v2 = (((0x6c796765 as libc::c_int as size_t) << 16 as libc::c_int)
        << 16 as libc::c_int)
        .wrapping_add(0x6e657261 as libc::c_int as size_t)
        ^ seed;
    v3 = (((0x74656462 as libc::c_int as size_t) << 16 as libc::c_int)
        << 16 as libc::c_int)
        .wrapping_add(0x79746573 as libc::c_int as size_t)
        ^ !seed;
    v0 = (v0 as libc::c_ulonglong
        ^ (0x706050403020100 as libc::c_ulonglong ^ seed as libc::c_ulonglong))
        as size_t;
    v1 = (v1 as libc::c_ulonglong
        ^ (0xf0e0d0c0b0a0908 as libc::c_ulonglong ^ !seed as libc::c_ulonglong))
        as size_t;
    v2 = (v2 as libc::c_ulonglong
        ^ (0x706050403020100 as libc::c_ulonglong ^ seed as libc::c_ulonglong))
        as size_t;
    v3 = (v3 as libc::c_ulonglong
        ^ (0xf0e0d0c0b0a0908 as libc::c_ulonglong ^ !seed as libc::c_ulonglong))
        as size_t;
    i = 0 as size_t;
    while i.wrapping_add(std::mem::size_of::<size_t>() as size_t) <= len {
        data = (*d.offset(0 as libc::c_int as isize) as libc::c_int
            | (*d.offset(1 as libc::c_int as isize) as libc::c_int)
                << 8 as libc::c_int
            | (*d.offset(2 as libc::c_int as isize) as libc::c_int)
                << 16 as libc::c_int
            | (*d.offset(3 as libc::c_int as isize) as libc::c_int)
                << 24 as libc::c_int) as size_t;
        data = (data as libc::c_ulong
            | ((((*d.offset(4 as libc::c_int as isize) as libc::c_int
                | (*d.offset(5 as libc::c_int as isize) as libc::c_int)
                    << 8 as libc::c_int
                | (*d.offset(6 as libc::c_int as isize) as libc::c_int)
                    << 16 as libc::c_int
                | (*d.offset(7 as libc::c_int as isize) as libc::c_int)
                    << 24 as libc::c_int) as size_t)
                << 16 as libc::c_int)
                << 16 as libc::c_int) as libc::c_ulong) as size_t;
        v3 = (v3 as libc::c_ulong ^ data as libc::c_ulong) as size_t;
        j = 0 as size_t;
        while j < STBDS_SIPHASH_C_ROUNDS as size_t {
            v0 = (v0 as libc::c_ulong).wrapping_add(v1 as libc::c_ulong) as size_t
                as size_t;
            v1 = v1 << 13 as libc::c_int | v1 >> STBDS_SIZE_T_BITS.wrapping_sub(13 as usize);
            v1 = (v1 as libc::c_ulong ^ v0 as libc::c_ulong) as size_t;
            v0 = v0
                << (std::mem::size_of::<size_t>() as usize)
                    .wrapping_mul(8 as usize)
                    .wrapping_div(2 as usize)
                | v0 >> STBDS_SIZE_T_BITS.wrapping_sub(
                    (std::mem::size_of::<size_t>() as usize)
                        .wrapping_mul(8 as usize)
                        .wrapping_div(2 as usize),
                );
            v2 = (v2 as libc::c_ulong).wrapping_add(v3 as libc::c_ulong) as size_t
                as size_t;
            v3 = v3 << 16 as libc::c_int | v3 >> STBDS_SIZE_T_BITS.wrapping_sub(16 as usize);
            v3 = (v3 as libc::c_ulong ^ v2 as libc::c_ulong) as size_t;
            v2 = (v2 as libc::c_ulong).wrapping_add(v1 as libc::c_ulong) as size_t
                as size_t;
            v1 = v1 << 17 as libc::c_int | v1 >> STBDS_SIZE_T_BITS.wrapping_sub(17 as usize);
            v1 = (v1 as libc::c_ulong ^ v2 as libc::c_ulong) as size_t;
            v2 = v2
                << (std::mem::size_of::<size_t>() as usize)
                    .wrapping_mul(8 as usize)
                    .wrapping_div(2 as usize)
                | v2 >> STBDS_SIZE_T_BITS.wrapping_sub(
                    (std::mem::size_of::<size_t>() as usize)
                        .wrapping_mul(8 as usize)
                        .wrapping_div(2 as usize),
                );
            v0 = (v0 as libc::c_ulong).wrapping_add(v3 as libc::c_ulong) as size_t
                as size_t;
            v3 = v3 << 21 as libc::c_int | v3 >> STBDS_SIZE_T_BITS.wrapping_sub(21 as usize);
            v3 = (v3 as libc::c_ulong ^ v0 as libc::c_ulong) as size_t;
            j = j.wrapping_add(1);
        }
        v0 = (v0 as libc::c_ulong ^ data as libc::c_ulong) as size_t;
        i = (i as libc::c_ulong)
            .wrapping_add(std::mem::size_of::<size_t>() as usize as libc::c_ulong)
            as size_t as size_t;
        d = d.offset(std::mem::size_of::<size_t>() as usize as isize);
    }
    data = len << STBDS_SIZE_T_BITS.wrapping_sub(8 as usize);
    let mut current_block_40: u64;
    match len.wrapping_sub(i) {
        7 => {
            data = (data as libc::c_ulong
                | (((*d.offset(6 as libc::c_int as isize) as size_t)
                    << 24 as libc::c_int)
                    << 24 as libc::c_int) as libc::c_ulong)
                as size_t;
            current_block_40 = 5368384839064798695;
        }
        6 => {
            current_block_40 = 5368384839064798695;
        }
        5 => {
            current_block_40 = 5542883694075707684;
        }
        4 => {
            current_block_40 = 958264337981697758;
        }
        3 => {
            current_block_40 = 2134191529824114813;
        }
        2 => {
            current_block_40 = 14172819746050043740;
        }
        1 => {
            current_block_40 = 14452068164804587099;
        }
        0 | _ => {
            current_block_40 = 1538046216550696469;
        }
    }
    match current_block_40 {
        5368384839064798695 => {
            data = (data as libc::c_ulong
                | (((*d.offset(5 as libc::c_int as isize) as size_t)
                    << 20 as libc::c_int)
                    << 20 as libc::c_int) as libc::c_ulong)
                as size_t;
            current_block_40 = 5542883694075707684;
        }
        _ => {}
    }
    match current_block_40 {
        5542883694075707684 => {
            data = (data as libc::c_ulong
                | (((*d.offset(4 as libc::c_int as isize) as size_t)
                    << 16 as libc::c_int)
                    << 16 as libc::c_int) as libc::c_ulong)
                as size_t;
            current_block_40 = 958264337981697758;
        }
        _ => {}
    }
    match current_block_40 {
        958264337981697758 => {
            data = (data as libc::c_ulong
                | ((*d.offset(3 as libc::c_int as isize) as libc::c_int)
                    << 24 as libc::c_int) as libc::c_ulong)
                as size_t;
            current_block_40 = 2134191529824114813;
        }
        _ => {}
    }
    match current_block_40 {
        2134191529824114813 => {
            data = (data as libc::c_ulong
                | ((*d.offset(2 as libc::c_int as isize) as libc::c_int)
                    << 16 as libc::c_int) as libc::c_ulong)
                as size_t;
            current_block_40 = 14172819746050043740;
        }
        _ => {}
    }
    match current_block_40 {
        14172819746050043740 => {
            data = (data as libc::c_ulong
                | ((*d.offset(1 as libc::c_int as isize) as libc::c_int)
                    << 8 as libc::c_int) as libc::c_ulong)
                as size_t;
            current_block_40 = 14452068164804587099;
        }
        _ => {}
    }
    match current_block_40 {
        14452068164804587099 => {
            data = (data as libc::c_ulong
                | *d.offset(0 as libc::c_int as isize) as libc::c_ulong)
                as size_t;
        }
        _ => {}
    }
    v3 = (v3 as libc::c_ulong ^ data as libc::c_ulong) as size_t;
    j = 0 as size_t;
    while j < STBDS_SIPHASH_C_ROUNDS as size_t {
        v0 = (v0 as libc::c_ulong).wrapping_add(v1 as libc::c_ulong) as size_t
            as size_t;
        v1 = v1 << 13 as libc::c_int | v1 >> STBDS_SIZE_T_BITS.wrapping_sub(13 as usize);
        v1 = (v1 as libc::c_ulong ^ v0 as libc::c_ulong) as size_t;
        v0 = v0
            << (std::mem::size_of::<size_t>() as usize)
                .wrapping_mul(8 as usize)
                .wrapping_div(2 as usize)
            | v0 >> STBDS_SIZE_T_BITS.wrapping_sub(
                (std::mem::size_of::<size_t>() as usize)
                    .wrapping_mul(8 as usize)
                    .wrapping_div(2 as usize),
            );
        v2 = (v2 as libc::c_ulong).wrapping_add(v3 as libc::c_ulong) as size_t
            as size_t;
        v3 = v3 << 16 as libc::c_int | v3 >> STBDS_SIZE_T_BITS.wrapping_sub(16 as usize);
        v3 = (v3 as libc::c_ulong ^ v2 as libc::c_ulong) as size_t;
        v2 = (v2 as libc::c_ulong).wrapping_add(v1 as libc::c_ulong) as size_t
            as size_t;
        v1 = v1 << 17 as libc::c_int | v1 >> STBDS_SIZE_T_BITS.wrapping_sub(17 as usize);
        v1 = (v1 as libc::c_ulong ^ v2 as libc::c_ulong) as size_t;
        v2 = v2
            << (std::mem::size_of::<size_t>() as usize)
                .wrapping_mul(8 as usize)
                .wrapping_div(2 as usize)
            | v2 >> STBDS_SIZE_T_BITS.wrapping_sub(
                (std::mem::size_of::<size_t>() as usize)
                    .wrapping_mul(8 as usize)
                    .wrapping_div(2 as usize),
            );
        v0 = (v0 as libc::c_ulong).wrapping_add(v3 as libc::c_ulong) as size_t
            as size_t;
        v3 = v3 << 21 as libc::c_int | v3 >> STBDS_SIZE_T_BITS.wrapping_sub(21 as usize);
        v3 = (v3 as libc::c_ulong ^ v0 as libc::c_ulong) as size_t;
        j = j.wrapping_add(1);
    }
    v0 = (v0 as libc::c_ulong ^ data as libc::c_ulong) as size_t;
    v2 = (v2 as libc::c_ulong ^ 0xff as libc::c_ulong) as size_t;
    j = 0 as size_t;
    while j < STBDS_SIPHASH_D_ROUNDS as size_t {
        v0 = (v0 as libc::c_ulong).wrapping_add(v1 as libc::c_ulong) as size_t
            as size_t;
        v1 = v1 << 13 as libc::c_int | v1 >> STBDS_SIZE_T_BITS.wrapping_sub(13 as usize);
        v1 = (v1 as libc::c_ulong ^ v0 as libc::c_ulong) as size_t;
        v0 = v0
            << (std::mem::size_of::<size_t>() as usize)
                .wrapping_mul(8 as usize)
                .wrapping_div(2 as usize)
            | v0 >> STBDS_SIZE_T_BITS.wrapping_sub(
                (std::mem::size_of::<size_t>() as usize)
                    .wrapping_mul(8 as usize)
                    .wrapping_div(2 as usize),
            );
        v2 = (v2 as libc::c_ulong).wrapping_add(v3 as libc::c_ulong) as size_t
            as size_t;
        v3 = v3 << 16 as libc::c_int | v3 >> STBDS_SIZE_T_BITS.wrapping_sub(16 as usize);
        v3 = (v3 as libc::c_ulong ^ v2 as libc::c_ulong) as size_t;
        v2 = (v2 as libc::c_ulong).wrapping_add(v1 as libc::c_ulong) as size_t
            as size_t;
        v1 = v1 << 17 as libc::c_int | v1 >> STBDS_SIZE_T_BITS.wrapping_sub(17 as usize);
        v1 = (v1 as libc::c_ulong ^ v2 as libc::c_ulong) as size_t;
        v2 = v2
            << (std::mem::size_of::<size_t>() as usize)
                .wrapping_mul(8 as usize)
                .wrapping_div(2 as usize)
            | v2 >> STBDS_SIZE_T_BITS.wrapping_sub(
                (std::mem::size_of::<size_t>() as usize)
                    .wrapping_mul(8 as usize)
                    .wrapping_div(2 as usize),
            );
        v0 = (v0 as libc::c_ulong).wrapping_add(v3 as libc::c_ulong) as size_t
            as size_t;
        v3 = v3 << 21 as libc::c_int | v3 >> STBDS_SIZE_T_BITS.wrapping_sub(21 as usize);
        v3 = (v3 as libc::c_ulong ^ v0 as libc::c_ulong) as size_t;
        j = j.wrapping_add(1);
    }
    return v0 ^ v1 ^ v2 ^ v3;
}
#[no_mangle]
pub unsafe extern "C" fn stbds_hash_bytes(
    mut p: *mut libc::c_void,
    mut len: size_t,
    mut seed: size_t,
) -> size_t {
    return stbds_siphash_bytes(p, len, seed);
}
unsafe extern "C" fn stbds_is_key_equal(
    mut a: *mut libc::c_void,
    mut elemsize: size_t,
    mut key: *mut libc::c_void,
    mut keysize: size_t,
    mut keyoffset: size_t,
    mut mode: libc::c_int,
    mut i: size_t,
) -> libc::c_int {
    if mode >= STBDS_HM_STRING {
        return (0 as libc::c_int
            == strcmp(
                key as *mut libc::c_char,
                *((a as *mut libc::c_char)
                    .offset(elemsize.wrapping_mul(i) as isize)
                    .offset(keyoffset as isize) as *mut *mut libc::c_char),
            )) as libc::c_int;
    } else {
        return (0 as libc::c_int
            == memcmp(
                key,
                (a as *mut libc::c_char)
                    .offset(elemsize.wrapping_mul(i) as isize)
                    .offset(keyoffset as isize) as *const libc::c_void,
                keysize,
            )) as libc::c_int;
    };
}
#[no_mangle]
pub unsafe extern "C" fn stbds_hmfree_func(mut a: *mut libc::c_void, mut elemsize: size_t) {
    if a.is_null() {
        return;
    }
    if !((*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).hash_table
        as *mut stbds_hash_index)
        .is_null()
    {
        if (*((*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize)))
            .hash_table as *mut stbds_hash_index))
            .string
            .mode as libc::c_int
            == STBDS_SH_STRDUP as libc::c_int
        {
            let mut i: size_t = 0;
            i = 1 as size_t;
            while i
                < (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize)))
                    .length
            {
                free(
                    *((a as *mut libc::c_char).offset(elemsize.wrapping_mul(i) as isize)
                        as *mut *mut libc::c_char)
                        as *mut libc::c_void,
                );
                i = i.wrapping_add(1);
            }
        }
        stbds_strreset(
            &raw mut (*((*(a as *mut stbds_array_header)
                .offset(-(1 as libc::c_int as isize)))
            .hash_table as *mut stbds_hash_index))
                .string,
        );
    }
    free((*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).hash_table);
    free(
        (a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))
            as *mut libc::c_void,
    );
}
unsafe extern "C" fn stbds_hm_find_slot(
    mut a: *mut libc::c_void,
    mut elemsize: size_t,
    mut key: *mut libc::c_void,
    mut keysize: size_t,
    mut keyoffset: size_t,
    mut mode: libc::c_int,
) -> ptrdiff_t {
    let mut raw_a: *mut libc::c_void =
        (a as *mut libc::c_char).offset(-(elemsize as isize)) as *mut libc::c_void;
    let mut table: *mut stbds_hash_index = (*(raw_a as *mut stbds_array_header)
        .offset(-(1 as libc::c_int as isize)))
    .hash_table as *mut stbds_hash_index;
    let mut hash: size_t = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut libc::c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step: size_t = STBDS_BUCKET_LENGTH as size_t;
    let mut limit: size_t = 0;
    let mut i: size_t = 0;
    let mut pos: size_t = 0;
    let mut bucket: *mut stbds_hash_bucket = std::ptr::null_mut::<stbds_hash_bucket>();
    if hash < 2 as size_t {
        hash = (hash as libc::c_ulong).wrapping_add(2 as libc::c_ulong) as size_t
            as size_t;
    }
    pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);
    loop {
        bucket = (*table).storage.offset(
            (pos >> (if STBDS_BUCKET_LENGTH == 8 as libc::c_int {
                3 as libc::c_int
            } else {
                2 as libc::c_int
            })) as isize,
        ) as *mut stbds_hash_bucket;
        i = pos & STBDS_BUCKET_MASK as size_t;
        while i < STBDS_BUCKET_LENGTH as size_t {
            if (*bucket).hash[i as usize] == hash {
                if stbds_is_key_equal(
                    a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    (*bucket).index[i as usize] as size_t,
                ) != 0
                {
                    return (pos & !STBDS_BUCKET_MASK as size_t).wrapping_add(i) as ptrdiff_t;
                }
            } else if (*bucket).hash[i as usize] == STBDS_HASH_EMPTY as size_t {
                return -(1 as libc::c_int) as ptrdiff_t;
            }
            i = i.wrapping_add(1);
        }
        limit = pos & STBDS_BUCKET_MASK as size_t;
        i = 0 as size_t;
        while i < limit {
            if (*bucket).hash[i as usize] == hash {
                if stbds_is_key_equal(
                    a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    (*bucket).index[i as usize] as size_t,
                ) != 0
                {
                    return (pos & !STBDS_BUCKET_MASK as size_t).wrapping_add(i) as ptrdiff_t;
                }
            } else if (*bucket).hash[i as usize] == STBDS_HASH_EMPTY as size_t {
                return -(1 as libc::c_int) as ptrdiff_t;
            }
            i = i.wrapping_add(1);
        }
        pos = (pos as libc::c_ulong).wrapping_add(step as libc::c_ulong) as size_t
            as size_t;
        step = (step as libc::c_ulong)
            .wrapping_add(STBDS_BUCKET_LENGTH as libc::c_ulong) as size_t
            as size_t;
        pos = (pos as libc::c_ulong
            & (*table).slot_count.wrapping_sub(1 as size_t) as libc::c_ulong)
            as size_t;
    }
}
#[no_mangle]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut a: *mut libc::c_void,
    mut elemsize: size_t,
    mut key: *mut libc::c_void,
    mut keysize: size_t,
    mut temp: *mut ptrdiff_t,
    mut mode: libc::c_int,
) -> *mut libc::c_void {
    let mut keyoffset: size_t = 0 as size_t;
    if a.is_null() {
        a = stbds_arrgrowf(
            std::ptr::null_mut::<libc::c_void>(),
            elemsize,
            0 as size_t,
            1 as size_t,
        );
        let ref mut fresh4 =
            (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).length;
        *fresh4 = (*fresh4 as libc::c_ulong).wrapping_add(1 as libc::c_ulong)
            as size_t as size_t;
        memset(a, 0 as libc::c_int, elemsize);
        *temp = STBDS_INDEX_EMPTY as ptrdiff_t;
        return (a as *mut libc::c_char).offset(elemsize as isize)
            as *mut libc::c_void;
    } else {
        let mut table: *mut stbds_hash_index = std::ptr::null_mut::<stbds_hash_index>();
        let mut raw_a: *mut libc::c_void = (a as *mut libc::c_char)
            .offset(-(elemsize as isize))
            as *mut libc::c_void;
        table = (*(raw_a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize)))
            .hash_table as *mut stbds_hash_index;
        if table.is_null() {
            *temp = -(1 as libc::c_int) as ptrdiff_t;
        } else {
            let mut slot: ptrdiff_t =
                stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 as ptrdiff_t {
                *temp = STBDS_INDEX_EMPTY as ptrdiff_t;
            } else {
                let mut b: *mut stbds_hash_bucket = (*table).storage.offset(
                    (slot
                        >> (if STBDS_BUCKET_LENGTH == 8 as libc::c_int {
                            3 as libc::c_int
                        } else {
                            2 as libc::c_int
                        })) as isize,
                ) as *mut stbds_hash_bucket;
                *temp = (*b).index[(slot & STBDS_BUCKET_MASK as ptrdiff_t) as usize];
            }
        }
        return a;
    };
}
#[no_mangle]
pub unsafe extern "C" fn stbds_hmget_key(
    mut a: *mut libc::c_void,
    mut elemsize: size_t,
    mut key: *mut libc::c_void,
    mut keysize: size_t,
    mut mode: libc::c_int,
) -> *mut libc::c_void {
    let mut temp: ptrdiff_t = 0;
    let mut p: *mut libc::c_void =
        stbds_hmget_key_ts(a, elemsize, key, keysize, &raw mut temp, mode);
    (*((p as *mut libc::c_char).offset(-(elemsize as isize)) as *mut stbds_array_header)
        .offset(-(1 as libc::c_int as isize)))
    .temp = temp;
    return p;
}
#[no_mangle]
pub unsafe extern "C" fn stbds_hmput_default(
    mut a: *mut libc::c_void,
    mut elemsize: size_t,
) -> *mut libc::c_void {
    if a.is_null()
        || (*((a as *mut libc::c_char).offset(-(elemsize as isize))
            as *mut stbds_array_header)
            .offset(-(1 as libc::c_int as isize)))
        .length
            == 0 as size_t
    {
        a = stbds_arrgrowf(
            (if !a.is_null() {
                (a as *mut libc::c_char).offset(-(elemsize as isize))
            } else {
                std::ptr::null_mut::<libc::c_char>()
            }) as *mut libc::c_void,
            elemsize,
            0 as size_t,
            1 as size_t,
        );
        let ref mut fresh5 =
            (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).length;
        *fresh5 = (*fresh5 as libc::c_ulong).wrapping_add(1 as libc::c_ulong)
            as size_t as size_t;
        memset(a, 0 as libc::c_int, elemsize);
        a = (a as *mut libc::c_char).offset(elemsize as isize) as *mut libc::c_void;
    }
    return a;
}
#[no_mangle]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut libc::c_void,
    mut elemsize: size_t,
    mut key: *mut libc::c_void,
    mut keysize: size_t,
    mut mode: libc::c_int,
) -> *mut libc::c_void {
    let mut keyoffset: size_t = 0 as size_t;
    let mut raw_a: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
    let mut table: *mut stbds_hash_index = std::ptr::null_mut::<stbds_hash_index>();
    if a.is_null() {
        a = stbds_arrgrowf(
            std::ptr::null_mut::<libc::c_void>(),
            elemsize,
            0 as size_t,
            1 as size_t,
        );
        memset(a, 0 as libc::c_int, elemsize);
        let ref mut fresh6 =
            (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).length;
        *fresh6 = (*fresh6 as libc::c_ulong).wrapping_add(1 as libc::c_ulong)
            as size_t as size_t;
        a = (a as *mut libc::c_char).offset(elemsize as isize) as *mut libc::c_void;
    }
    raw_a = a;
    a = (a as *mut libc::c_char).offset(-(elemsize as isize)) as *mut libc::c_void;
    table = (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).hash_table
        as *mut stbds_hash_index;
    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let mut nt: *mut stbds_hash_index = std::ptr::null_mut::<stbds_hash_index>();
        let mut slot_count: size_t = 0;
        slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH as size_t
        } else {
            (*table).slot_count.wrapping_mul(2 as size_t)
        };
        nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            free(table as *mut libc::c_void);
        } else {
            (*nt).string.mode = (if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT as libc::c_int
            } else {
                0 as libc::c_int
            }) as libc::c_uchar;
        }
        table = nt;
        let ref mut fresh7 = (*(a as *mut stbds_array_header)
            .offset(-(1 as libc::c_int as isize)))
        .hash_table;
        *fresh7 = table as *mut libc::c_void;
    }
    let mut hash: size_t = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut libc::c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step: size_t = STBDS_BUCKET_LENGTH as size_t;
    let mut pos: size_t = 0;
    let mut tombstone: ptrdiff_t = -(1 as libc::c_int) as ptrdiff_t;
    let mut bucket: *mut stbds_hash_bucket = std::ptr::null_mut::<stbds_hash_bucket>();
    if hash < 2 as size_t {
        hash = (hash as libc::c_ulong).wrapping_add(2 as libc::c_ulong) as size_t
            as size_t;
    }
    pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);
    's_101: loop {
        let mut limit: size_t = 0;
        let mut i: size_t = 0;
        bucket = (*table).storage.offset(
            (pos >> (if STBDS_BUCKET_LENGTH == 8 as libc::c_int {
                3 as libc::c_int
            } else {
                2 as libc::c_int
            })) as isize,
        ) as *mut stbds_hash_bucket;
        i = pos & STBDS_BUCKET_MASK as size_t;
        while i < STBDS_BUCKET_LENGTH as size_t {
            if (*bucket).hash[i as usize] == hash {
                if stbds_is_key_equal(
                    raw_a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    (*bucket).index[i as usize] as size_t,
                ) != 0
                {
                    (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize)))
                        .temp = (*bucket).index[i as usize];
                    if mode >= STBDS_HM_STRING {
                        let ref mut fresh8 = *((*(a as *mut stbds_array_header)
                            .offset(-(1 as libc::c_int as isize)))
                        .hash_table
                            as *mut *mut libc::c_char);
                        *fresh8 = *((raw_a as *mut libc::c_char)
                            .offset(elemsize.wrapping_mul((*bucket).index[i as usize] as size_t)
                                as isize)
                            .offset(keyoffset as isize)
                            as *mut *mut libc::c_char);
                    }
                    return (a as *mut libc::c_char).offset(elemsize as isize)
                        as *mut libc::c_void;
                }
            } else if (*bucket).hash[i as usize] == 0 as size_t {
                pos = (pos & !STBDS_BUCKET_MASK as size_t).wrapping_add(i);
                break 's_101;
            } else if tombstone < 0 as ptrdiff_t {
                if (*bucket).index[i as usize] == STBDS_INDEX_DELETED as ptrdiff_t {
                    tombstone = (pos & !STBDS_BUCKET_MASK as size_t).wrapping_add(i) as ptrdiff_t;
                }
            }
            i = i.wrapping_add(1);
        }
        limit = pos & STBDS_BUCKET_MASK as size_t;
        i = 0 as size_t;
        while i < limit {
            if (*bucket).hash[i as usize] == hash {
                if stbds_is_key_equal(
                    raw_a,
                    elemsize,
                    key,
                    keysize,
                    keyoffset,
                    mode,
                    (*bucket).index[i as usize] as size_t,
                ) != 0
                {
                    (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize)))
                        .temp = (*bucket).index[i as usize];
                    return (a as *mut libc::c_char).offset(elemsize as isize)
                        as *mut libc::c_void;
                }
            } else if (*bucket).hash[i as usize] == 0 as size_t {
                pos = (pos & !STBDS_BUCKET_MASK as size_t).wrapping_add(i);
                break 's_101;
            } else if tombstone < 0 as ptrdiff_t {
                if (*bucket).index[i as usize] == STBDS_INDEX_DELETED as ptrdiff_t {
                    tombstone = (pos & !STBDS_BUCKET_MASK as size_t).wrapping_add(i) as ptrdiff_t;
                }
            }
            i = i.wrapping_add(1);
        }
        pos = (pos as libc::c_ulong).wrapping_add(step as libc::c_ulong) as size_t
            as size_t;
        step = (step as libc::c_ulong)
            .wrapping_add(STBDS_BUCKET_LENGTH as libc::c_ulong) as size_t
            as size_t;
        pos = (pos as libc::c_ulong
            & (*table).slot_count.wrapping_sub(1 as size_t) as libc::c_ulong)
            as size_t;
    }
    if tombstone >= 0 as ptrdiff_t {
        pos = tombstone as size_t;
        (*table).tombstone_count = (*table).tombstone_count.wrapping_sub(1);
    }
    (*table).used_count = (*table).used_count.wrapping_add(1);
    let mut i_0: ptrdiff_t = if !a.is_null() {
        (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).length
            as ptrdiff_t
    } else {
        0 as ptrdiff_t
    };
    if (i_0 as size_t).wrapping_add(1 as size_t)
        > (if !a.is_null() {
            (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).capacity
        } else {
            0 as size_t
        })
    {
        let ref mut fresh9 = *&raw mut a;
        *fresh9 = stbds_arrgrowf(a, elemsize, 1 as size_t, 0 as size_t);
    }
    raw_a = (a as *mut libc::c_char).offset(elemsize as isize) as *mut libc::c_void;
    '_c2rust_label: {
        if (i_0 as size_t).wrapping_add(1 as size_t)
            <= (if !a.is_null() {
                (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize)))
                    .capacity
            } else {
                0 as size_t
            })
        {
        } else {
            __assert_fail(
                b"(size_t) i+1 <= stbds_arrcap(a)\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-LWI3Rj/driver/c_src/src/lib.c\0" as *const u8
                    as *const libc::c_char,
                778 as libc::c_uint,
                b"void *stbds_hmput_key(void *, size_t, void *, size_t, int)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).length =
        (i_0 + 1 as ptrdiff_t) as size_t;
    bucket = (*table).storage.offset(
        (pos >> (if STBDS_BUCKET_LENGTH == 8 as libc::c_int {
            3 as libc::c_int
        } else {
            2 as libc::c_int
        })) as isize,
    ) as *mut stbds_hash_bucket;
    (*bucket).hash[(pos & STBDS_BUCKET_MASK as size_t) as usize] = hash;
    (*bucket).index[(pos & STBDS_BUCKET_MASK as size_t) as usize] = i_0 - 1 as ptrdiff_t;
    (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).temp =
        i_0 - 1 as ptrdiff_t;
    match (*table).string.mode as libc::c_int {
        2 => {
            let ref mut fresh10 = *((a as *mut libc::c_char)
                .offset(elemsize.wrapping_mul(i_0 as size_t) as isize)
                as *mut *mut libc::c_char);
            *fresh10 = stbds_strdup(key as *mut libc::c_char);
            let ref mut fresh11 = *((*(a as *mut stbds_array_header)
                .offset(-(1 as libc::c_int as isize)))
            .hash_table as *mut *mut libc::c_char);
            *fresh11 = *fresh10;
        }
        3 => {
            let ref mut fresh12 = *((a as *mut libc::c_char)
                .offset(elemsize.wrapping_mul(i_0 as size_t) as isize)
                as *mut *mut libc::c_char);
            *fresh12 = stbds_stralloc(&raw mut (*table).string, key as *mut libc::c_char);
            let ref mut fresh13 = *((*(a as *mut stbds_array_header)
                .offset(-(1 as libc::c_int as isize)))
            .hash_table as *mut *mut libc::c_char);
            *fresh13 = *fresh12;
        }
        1 => {
            let ref mut fresh14 = *((a as *mut libc::c_char)
                .offset(elemsize.wrapping_mul(i_0 as size_t) as isize)
                as *mut *mut libc::c_char);
            *fresh14 = key as *mut libc::c_char;
            let ref mut fresh15 = *((*(a as *mut stbds_array_header)
                .offset(-(1 as libc::c_int as isize)))
            .hash_table as *mut *mut libc::c_char);
            *fresh15 = *fresh14;
        }
        _ => {
            memcpy(
                (a as *mut libc::c_char)
                    .offset(elemsize.wrapping_mul(i_0 as size_t) as isize)
                    as *mut libc::c_void,
                key,
                keysize,
            );
        }
    }
    return (a as *mut libc::c_char).offset(elemsize as isize) as *mut libc::c_void;
}
#[no_mangle]
pub unsafe extern "C" fn stbds_shmode_func(
    mut elemsize: size_t,
    mut mode: libc::c_int,
) -> *mut libc::c_void {
    let mut a: *mut libc::c_void = stbds_arrgrowf(
        std::ptr::null_mut::<libc::c_void>(),
        elemsize,
        0 as size_t,
        1 as size_t,
    );
    let mut h: *mut stbds_hash_index = std::ptr::null_mut::<stbds_hash_index>();
    memset(a, 0 as libc::c_int, elemsize);
    (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).length =
        1 as size_t;
    h = stbds_make_hash_index(
        STBDS_BUCKET_LENGTH as size_t,
        std::ptr::null_mut::<stbds_hash_index>(),
    );
    let ref mut fresh19 =
        (*(a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).hash_table;
    *fresh19 = h as *mut libc::c_void;
    (*h).string.mode = mode as libc::c_uchar;
    return (a as *mut libc::c_char).offset(elemsize as isize) as *mut libc::c_void;
}
#[no_mangle]
pub unsafe extern "C" fn stbds_hmdel_key(
    mut a: *mut libc::c_void,
    mut elemsize: size_t,
    mut key: *mut libc::c_void,
    mut keysize: size_t,
    mut keyoffset: size_t,
    mut mode: libc::c_int,
) -> *mut libc::c_void {
    if a.is_null() {
        return std::ptr::null_mut::<libc::c_void>();
    } else {
        let mut table: *mut stbds_hash_index = std::ptr::null_mut::<stbds_hash_index>();
        let mut raw_a: *mut libc::c_void = (a as *mut libc::c_char)
            .offset(-(elemsize as isize))
            as *mut libc::c_void;
        table = (*(raw_a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize)))
            .hash_table as *mut stbds_hash_index;
        (*(raw_a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).temp =
            0 as ptrdiff_t;
        if table.is_null() {
            return a;
        } else {
            let mut slot: ptrdiff_t = 0;
            slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 as ptrdiff_t {
                return a;
            } else {
                let mut b: *mut stbds_hash_bucket = (*table).storage.offset(
                    (slot
                        >> (if STBDS_BUCKET_LENGTH == 8 as libc::c_int {
                            3 as libc::c_int
                        } else {
                            2 as libc::c_int
                        })) as isize,
                ) as *mut stbds_hash_bucket;
                let mut i: libc::c_int =
                    (slot & STBDS_BUCKET_MASK as ptrdiff_t) as libc::c_int;
                let mut old_index: ptrdiff_t = (*b).index[i as usize];
                let mut final_index: ptrdiff_t = (if !raw_a.is_null() {
                    (*(raw_a as *mut stbds_array_header)
                        .offset(-(1 as libc::c_int as isize)))
                    .length as ptrdiff_t
                } else {
                    0 as ptrdiff_t
                }) - 1 as ptrdiff_t
                    - 1 as ptrdiff_t;
                '_c2rust_label: {
                    if slot < (*table).slot_count as ptrdiff_t {
                    } else {
                        __assert_fail(
                            b"slot < (ptrdiff_t) table->slot_count\0" as *const u8
                                as *const libc::c_char,
                            b"/tmp/harvest-translate-LWI3Rj/driver/c_src/src/lib.c\0"
                                as *const u8
                                as *const libc::c_char,
                            828 as libc::c_uint,
                            b"void *stbds_hmdel_key(void *, size_t, void *, size_t, size_t, int)\0"
                                as *const u8
                                as *const libc::c_char,
                        );
                    }
                };
                (*table).used_count = (*table).used_count.wrapping_sub(1);
                (*table).tombstone_count = (*table).tombstone_count.wrapping_add(1);
                (*(raw_a as *mut stbds_array_header).offset(-(1 as libc::c_int as isize)))
                    .temp = 1 as ptrdiff_t;
                '_c2rust_label_0: {
                    if (*table).used_count >= 0 as size_t {
                    } else {
                        __assert_fail(
                            b"table->used_count >= 0\0" as *const u8 as *const libc::c_char,
                            b"/tmp/harvest-translate-LWI3Rj/driver/c_src/src/lib.c\0"
                                as *const u8
                                as *const libc::c_char,
                            832 as libc::c_uint,
                            b"void *stbds_hmdel_key(void *, size_t, void *, size_t, size_t, int)\0"
                                as *const u8
                                as *const libc::c_char,
                        );
                    }
                };
                (*b).hash[i as usize] = STBDS_HASH_DELETED as size_t;
                (*b).index[i as usize] = STBDS_INDEX_DELETED as ptrdiff_t;
                if mode == STBDS_HM_STRING
                    && (*table).string.mode as libc::c_int
                        == STBDS_SH_STRDUP as libc::c_int
                {
                    free(
                        *((a as *mut libc::c_char)
                            .offset(elemsize.wrapping_mul(old_index as size_t) as isize)
                            as *mut *mut libc::c_char)
                            as *mut libc::c_void,
                    );
                }
                if old_index != final_index {
                    memmove(
                        (a as *mut libc::c_char)
                            .offset(elemsize.wrapping_mul(old_index as size_t) as isize)
                            as *mut libc::c_void,
                        (a as *mut libc::c_char)
                            .offset(elemsize.wrapping_mul(final_index as size_t) as isize)
                            as *const libc::c_void,
                        elemsize,
                    );
                    if mode == STBDS_HM_STRING {
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            *((a as *mut libc::c_char)
                                .offset(elemsize.wrapping_mul(old_index as size_t) as isize)
                                .offset(keyoffset as isize)
                                as *mut *mut libc::c_char)
                                as *mut libc::c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    } else {
                        slot = stbds_hm_find_slot(
                            a,
                            elemsize,
                            (a as *mut libc::c_char)
                                .offset(elemsize.wrapping_mul(old_index as size_t) as isize)
                                .offset(keyoffset as isize)
                                as *mut libc::c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                    }
                    '_c2rust_label_1: {
                        if slot >= 0 as ptrdiff_t {
                        } else {
                            __assert_fail(
                                b"slot >= 0\0" as *const u8 as *const libc::c_char,
                                b"/tmp/harvest-translate-LWI3Rj/driver/c_src/src/lib.c\0"
                                    as *const u8 as *const libc::c_char,
                                846 as libc::c_uint,
                                b"void *stbds_hmdel_key(void *, size_t, void *, size_t, size_t, int)\0"
                                    as *const u8 as *const libc::c_char,
                            );
                        }
                    };
                    b = (*table).storage.offset(
                        (slot
                            >> (if STBDS_BUCKET_LENGTH == 8 as libc::c_int {
                                3 as libc::c_int
                            } else {
                                2 as libc::c_int
                            })) as isize,
                    ) as *mut stbds_hash_bucket;
                    i = (slot & STBDS_BUCKET_MASK as ptrdiff_t) as libc::c_int;
                    '_c2rust_label_2: {
                        if (*b).index[i as usize] == final_index {
                        } else {
                            __assert_fail(
                                b"b->index[i] == final_index\0" as *const u8
                                    as *const libc::c_char,
                                b"/tmp/harvest-translate-LWI3Rj/driver/c_src/src/lib.c\0"
                                    as *const u8 as *const libc::c_char,
                                849 as libc::c_uint,
                                b"void *stbds_hmdel_key(void *, size_t, void *, size_t, size_t, int)\0"
                                    as *const u8 as *const libc::c_char,
                            );
                        }
                    };
                    (*b).index[i as usize] = old_index;
                }
                let ref mut fresh16 = (*(raw_a as *mut stbds_array_header)
                    .offset(-(1 as libc::c_int as isize)))
                .length;
                *fresh16 = (*fresh16 as libc::c_ulong)
                    .wrapping_sub(1 as libc::c_ulong) as size_t
                    as size_t;
                if (*table).used_count < (*table).used_count_shrink_threshold
                    && (*table).slot_count > STBDS_BUCKET_LENGTH as size_t
                {
                    let ref mut fresh17 = (*(raw_a as *mut stbds_array_header)
                        .offset(-(1 as libc::c_int as isize)))
                    .hash_table;
                    *fresh17 = stbds_make_hash_index(
                        (*table).slot_count >> 1 as libc::c_int,
                        table,
                    ) as *mut libc::c_void;
                    free(table as *mut libc::c_void);
                } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
                    let ref mut fresh18 = (*(raw_a as *mut stbds_array_header)
                        .offset(-(1 as libc::c_int as isize)))
                    .hash_table;
                    *fresh18 = stbds_make_hash_index((*table).slot_count, table)
                        as *mut libc::c_void;
                    free(table as *mut libc::c_void);
                }
                return a;
            }
        }
    };
}
unsafe extern "C" fn stbds_strdup(mut str: *mut libc::c_char) -> *mut libc::c_char {
    let mut len: size_t = strlen(str).wrapping_add(1 as size_t);
    let mut p: *mut libc::c_char =
        realloc(std::ptr::null_mut::<libc::c_void>(), len) as *mut libc::c_char;
    memmove(
        p as *mut libc::c_void,
        str as *const libc::c_void,
        len,
    );
    return p;
}
pub const STBDS_STRING_ARENA_BLOCKSIZE_MIN: libc::c_uint = 512 as libc::c_uint;
pub const STBDS_STRING_ARENA_BLOCKSIZE_MAX: libc::c_uint =
    (1 as libc::c_uint) << 20 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn stbds_stralloc(
    mut a: *mut stbds_string_arena,
    mut str: *mut libc::c_char,
) -> *mut libc::c_char {
    let mut p: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let mut len: size_t = strlen(str).wrapping_add(1 as size_t);
    if len > (*a).remaining {
        let mut blocksize: size_t = (*a).block as size_t;
        blocksize =
            (512 as libc::c_uint as size_t) << (blocksize >> 1 as libc::c_int);
        if blocksize < ((1 as libc::c_uint) << 20 as libc::c_int) as size_t {
            (*a).block = (*a).block.wrapping_add(1);
        }
        if len > blocksize {
            let mut sb: *mut stbds_string_block = realloc(
                std::ptr::null_mut::<libc::c_void>(),
                (std::mem::size_of::<stbds_string_block>() as size_t)
                    .wrapping_sub(8 as size_t)
                    .wrapping_add(len),
            ) as *mut stbds_string_block;
            memmove(
                &raw mut (*sb).storage as *mut libc::c_char as *mut libc::c_void,
                str as *const libc::c_void,
                len,
            );
            if !(*a).storage.is_null() {
                (*sb).next = (*(*a).storage).next;
                (*(*a).storage).next = sb as *mut stbds_string_block;
            } else {
                (*sb).next = std::ptr::null_mut::<stbds_string_block>();
                (*a).storage = sb;
                (*a).remaining = 0 as size_t;
            }
            return &raw mut (*sb).storage as *mut libc::c_char;
        } else {
            let mut sb_0: *mut stbds_string_block = realloc(
                std::ptr::null_mut::<libc::c_void>(),
                (std::mem::size_of::<stbds_string_block>() as size_t)
                    .wrapping_sub(8 as size_t)
                    .wrapping_add(blocksize),
            ) as *mut stbds_string_block;
            (*sb_0).next = (*a).storage as *mut stbds_string_block;
            (*a).storage = sb_0;
            (*a).remaining = blocksize;
        }
    }
    '_c2rust_label: {
        if len <= (*a).remaining {
        } else {
            __assert_fail(
                b"len <= a->remaining\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-LWI3Rj/driver/c_src/src/lib.c\0" as *const u8
                    as *const libc::c_char,
                913 as libc::c_uint,
                b"char *stbds_stralloc(stbds_string_arena *, char *)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    p = (&raw mut (*(*a).storage).storage as *mut libc::c_char)
        .offset((*a).remaining as isize)
        .offset(-(len as isize));
    (*a).remaining = ((*a).remaining as libc::c_ulong)
        .wrapping_sub(len as libc::c_ulong) as size_t as size_t;
    memmove(
        p as *mut libc::c_void,
        str as *const libc::c_void,
        len,
    );
    return p;
}
#[no_mangle]
pub unsafe extern "C" fn stbds_strreset(mut a: *mut stbds_string_arena) {
    let mut x: *mut stbds_string_block = std::ptr::null_mut::<stbds_string_block>();
    let mut y: *mut stbds_string_block = std::ptr::null_mut::<stbds_string_block>();
    x = (*a).storage;
    while !x.is_null() {
        y = (*x).next as *mut stbds_string_block;
        free(x as *mut libc::c_void);
        x = y;
    }
    memset(
        a as *mut libc::c_void,
        0 as libc::c_int,
        std::mem::size_of::<stbds_string_arena>() as size_t,
    );
}
static mut buffer: [libc::c_char; 256] = [0; 256];
#[no_mangle]
pub unsafe extern "C" fn strkey(mut n: libc::c_int) -> *mut libc::c_char {
    sprintf(
        &raw mut buffer as *mut libc::c_char,
        b"test_%d\0" as *const u8 as *const libc::c_char,
        n,
    );
    return &raw mut buffer as *mut libc::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn arr_push(mut num: libc::c_int) {
    let mut arr: *mut libc::c_int = std::ptr::null_mut::<libc::c_int>();
    let mut i: libc::c_int = 0;
    let mut j: libc::c_int = 0;
    '_c2rust_label: {
        if (if !arr.is_null() {
            (*(arr as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))).length
                as ptrdiff_t
        } else {
            0 as ptrdiff_t
        }) == 0 as ptrdiff_t
        {
        } else {
            __assert_fail(
                b"arrlen(arr)==0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-LWI3Rj/driver/c_src/src/lib.c\0" as *const u8
                    as *const libc::c_char,
                950 as libc::c_uint,
                __ASSERT_FUNCTION.as_ptr(),
            );
        }
    };
    i = 0 as libc::c_int;
    while i < num {
        j = 0 as libc::c_int;
        while j < i {
            if arr.is_null()
                || (*(arr as *mut stbds_array_header).offset(-(1 as libc::c_int as isize)))
                    .length
                    .wrapping_add(1 as size_t)
                    > (*(arr as *mut stbds_array_header)
                        .offset(-(1 as libc::c_int as isize)))
                    .capacity
            {
                arr = stbds_arrgrowf(
                    arr as *mut libc::c_void,
                    std::mem::size_of::<libc::c_int>() as size_t,
                    1 as size_t,
                    0 as size_t,
                ) as *mut libc::c_int;
            } else {
            };
            let ref mut fresh0 = (*(arr as *mut stbds_array_header)
                .offset(-(1 as libc::c_int as isize)))
            .length;
            let fresh1 = *fresh0;
            *fresh0 = (*fresh0).wrapping_add(1);
            *arr.offset(fresh1 as isize) = j;
            j += 1;
        }
        if !arr.is_null() {
            free(
                (arr as *mut stbds_array_header).offset(-(1 as libc::c_int as isize))
                    as *mut libc::c_void,
            );
        } else {
        };
        arr = std::ptr::null_mut::<libc::c_int>();
        i += 50 as libc::c_int;
    }
}
