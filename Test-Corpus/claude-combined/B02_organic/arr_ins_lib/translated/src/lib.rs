#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::c_void;
use std::os::raw::c_int;
use std::ptr;

// Constants
const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3; // log2(8)
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

#[inline]
fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

#[allow(dead_code)]
const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: u8 = 0;
#[allow(dead_code)]
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_SIZE_T_BITS: u32 = (std::mem::size_of::<usize>() as u32) * 8;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

// Structures
#[repr(C)]
#[derive(Copy, Clone)]
pub struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [u8; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
pub struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
pub struct stbds_hash_bucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
pub struct stbds_hash_index {
    temp_key: *mut u8,
    slot_count: usize,
    used_count: usize,
    used_count_threshold: usize,
    used_count_shrink_threshold: usize,
    tombstone_count: usize,
    tombstone_count_threshold: usize,
    seed: usize,
    slot_count_log2: usize,
    string: stbds_string_arena,
    storage: *mut stbds_hash_bucket,
}

// Memory helpers (use libc realloc/free to match C exactly)
#[inline]
unsafe fn stbds_realloc(p: *mut c_void, s: usize) -> *mut c_void {
    libc::realloc(p, s)
}

#[inline]
unsafe fn stbds_free(p: *mut c_void) {
    libc::free(p)
}

#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

#[inline]
unsafe fn stbds_arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

#[inline]
unsafe fn stbds_arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

#[inline]
unsafe fn stbds_temp(a: *mut c_void) -> *mut isize {
    &mut (*stbds_header(a)).temp as *mut isize
}

#[inline]
unsafe fn stbds_temp_key(a: *mut c_void) -> *mut *mut u8 {
    &mut (*stbds_header(a)).hash_table as *mut *mut c_void as *mut *mut u8
}

#[inline]
unsafe fn stbds_hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).offset(-(elemsize as isize)) as *mut c_void
}

#[inline]
unsafe fn stbds_arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).add(elemsize) as *mut c_void
}

#[inline]
unsafe fn stbds_hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

#[inline]
fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

#[inline]
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline]
fn stbds_rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

// Global hash seed
static mut STBDS_HASH_SEED: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED = seed;
}

// stbds_arrgrowf
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= stbds_arrcap(a) {
        return a;
    }

    if min_cap < 2 * stbds_arrcap(a) {
        min_cap = 2 * stbds_arrcap(a);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let prev_header = if a.is_null() {
        ptr::null_mut::<c_void>()
    } else {
        stbds_header(a) as *mut c_void
    };

    let total_size = elemsize * min_cap + std::mem::size_of::<stbds_array_header>();
    let b_raw = stbds_realloc(prev_header, total_size);
    let b = (b_raw as *mut u8).add(std::mem::size_of::<stbds_array_header>()) as *mut c_void;

    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;

    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    stbds_free(stbds_header(a) as *mut c_void);
}

// hash string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut u8, seed: usize) -> usize {
    let mut hash = seed;
    let mut p = str_;
    while *p != 0 {
        hash = stbds_rotate_left(hash, 9).wrapping_add(*p as usize);
        p = p.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ stbds_rotate_right(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ stbds_rotate_right(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= stbds_rotate_right(hash, 22);
    hash.wrapping_add(seed)
}

// SipHash
#[inline]
fn sipround(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = stbds_rotate_left(*v1, 13);
    *v1 ^= *v0;
    *v0 = stbds_rotate_left(*v0, STBDS_SIZE_T_BITS / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = stbds_rotate_left(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = stbds_rotate_left(*v1, 17);
    *v1 ^= *v2;
    *v2 = stbds_rotate_left(*v2, STBDS_SIZE_T_BITS / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = stbds_rotate_left(*v3, 21);
    *v3 ^= *v0;
}

unsafe fn stbds_siphash_bytes(p: *const c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    v0 = ((((0x736f6d65usize) << 16) << 16).wrapping_add(0x70736575)) ^ seed;
    v1 = ((((0x646f7261usize) << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    v2 = ((((0x6c796765usize) << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
    v3 = ((((0x74656462usize) << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let mut i = 0usize;
    let elemsize = std::mem::size_of::<usize>();
    while i + elemsize <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        // In C, d[i]<<24 is computed as int (since d[i] is unsigned char which is promoted to int).
        // The result is then assigned to size_t (usize). On 64-bit, sign extension would happen if bit31 set.
        // Actually wait: In C, unsigned char * 1 << 24 - the unsigned char promotes to int. (d[3] << 24) is an int.
        // Then it's bitwise OR'd with data which is size_t. The int is converted to size_t.
        // Conversion of negative int to size_t = sign-extension on most systems? No, actually:
        // C says: For unsigned int conversion of negative signed int to unsigned, mod 2^N is added.
        // So if int is 32-bit and negative, conversion to 64-bit unsigned: sign-extends?
        // Actually no. C says: "the value is converted by repeatedly adding or subtracting one more than the maximum value that can be represented in the new type until the value is in the range of the new type."
        // So negative int -> size_t: adds 2^64. So a negative int X becomes 2^64 + X = sign-extended.
        // So d[3] << 24, if bit 31 set, becomes a negative int, then sign-extends to size_t.
        // For example: d[3]=0x80, (0x80 << 24) = 0x80000000 as signed int = -2147483648.
        // As size_t: 0xffffffff80000000.
        // So we need to replicate this sign-extension behavior!
        let d0 = *d as i32;
        let d1 = *d.add(1) as i32;
        let d2 = *d.add(2) as i32;
        let d3 = *d.add(3) as i32;
        let lower = d0 | (d1 << 8) | (d2 << 16) | (d3 << 24);
        // sign-extend lower (i32) to usize
        data = lower as isize as usize;
        // |= ((size_t) (d4 | (d5 << 8) | (d6 << 16) | (d7 << 24))) << 16 << 16
        let d4 = *d.add(4) as i32;
        let d5 = *d.add(5) as i32;
        let d6 = *d.add(6) as i32;
        let d7 = *d.add(7) as i32;
        let upper_int = d4 | (d5 << 8) | (d6 << 16) | (d7 << 24);
        // First cast to size_t (sign-extend), then shift
        let upper_usize = upper_int as isize as usize;
        data |= (upper_usize << 16) << 16;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i += elemsize;
        d = d.add(elemsize);
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    // switch with fallthroughs
    let rem = len - i;
    // The C switch falls through, accumulating bytes
    if rem >= 7 {
        data |= ((*d.add(6) as usize) << 24) << 24;
    }
    if rem >= 6 {
        data |= ((*d.add(5) as usize) << 20) << 20;
    }
    if rem >= 5 {
        data |= ((*d.add(4) as usize) << 16) << 16;
    }
    if rem >= 4 {
        // (d[3] << 24) - this is an int, then ORed into size_t. Sign-extends.
        let d3 = *d.add(3) as i32;
        let v = (d3 << 24) as isize as usize;
        data |= v;
    }
    if rem >= 3 {
        data |= (*d.add(2) as usize) << 16;
    }
    if rem >= 2 {
        data |= (*d.add(1) as usize) << 8;
    }
    if rem >= 1 {
        data |= *d as usize;
    }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p as *const c_void, len, seed)
}

// stbds_log2
fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

// probe position
#[inline]
fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

// stbds_load_32_or_64 macro implementation
fn stbds_load_32_or_64(v32: u64, v64_hi: u64, v64_lo: u64) -> usize {
    // temp = v64_lo ^ v32, temp <<= 16, temp <<= 16, temp >>= 16, temp >>= 16
    // var = v64_hi, var <<= 16, var <<= 16
    // var ^= temp ^ v32
    // The shifts are all on size_t. On 64-bit: temp <<= 16, temp <<= 16 = << 32; >>= 16, >>= 16 = >> 32. So temp = (v64_lo ^ v32) & 0xFFFFFFFF effectively, but with the upper bits cleared (since shifts are unsigned).
    // Actually: x <<= 16 << 16 then >>= 16 >>= 16. On 32-bit size_t, x <<= 16 twice would shift out the bits; >>= 16 twice would just give 0. So this is the trick to handle both 32 and 64-bit.
    // On 64-bit:
    let mut temp: usize = (v64_lo as usize) ^ (v32 as usize);
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    let mut var: usize = v64_hi as usize;
    var <<= 16;
    var <<= 16;
    var ^= temp ^ (v32 as usize);
    var
}

// stbds_make_hash_index
unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let alloc_size = (slot_count >> STBDS_BUCKET_SHIFT) * std::mem::size_of::<stbds_hash_bucket>()
        + std::mem::size_of::<stbds_hash_index>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let t = stbds_realloc(ptr::null_mut(), alloc_size) as *mut stbds_hash_index;
    let storage_addr = t.add(1) as usize;
    (*t).storage = stbds_align_fwd(storage_addr, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
    (*t).slot_count = slot_count;
    (*t).slot_count_log2 = stbds_log2(slot_count);
    (*t).tombstone_count = 0;
    (*t).used_count = 0;

    (*t).used_count_threshold = slot_count - (slot_count >> 2);
    (*t).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);
    (*t).used_count_shrink_threshold = slot_count >> 2;

    if slot_count <= STBDS_BUCKET_LENGTH {
        (*t).used_count_shrink_threshold = 0;
    }

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        ptr::write_bytes(&mut (*t).string as *mut stbds_string_arena, 0, 1);
        (*t).seed = STBDS_HASH_SEED;
        let a = stbds_load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b = stbds_load_32_or_64(715136305, 0, 0xb504f32d);
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    {
        for i in 0..(slot_count >> STBDS_BUCKET_SHIFT) {
            let bucket = (*t).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                (*bucket).hash[j] = STBDS_HASH_EMPTY;
            }
            for j in 0..STBDS_BUCKET_LENGTH {
                (*bucket).index[j] = STBDS_INDEX_EMPTY;
            }
        }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        for i in 0..((*ot).slot_count >> STBDS_BUCKET_SHIFT) {
            let ob = (*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if stbds_index_in_use((*ob).index[j]) {
                    let hash = (*ob).hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'rehash_loop: loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);

                        let start = pos & STBDS_BUCKET_MASK;
                        let mut placed = false;
                        for z in start..STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                placed = true;
                                break;
                            }
                        }
                        if placed {
                            break 'rehash_loop;
                        }

                        let limit = pos & STBDS_BUCKET_MASK;
                        for z in 0..limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                placed = true;
                                break;
                            }
                        }
                        if placed {
                            break 'rehash_loop;
                        }

                        pos = pos.wrapping_add(step);
                        step += STBDS_BUCKET_LENGTH;
                        pos &= (*t).slot_count - 1;
                    }
                }
            }
        }
    }

    t
}

unsafe fn stbds_is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: isize,
) -> bool {
    let item_ptr = (a as *mut u8)
        .add(elemsize.wrapping_mul(i as usize))
        .add(keyoffset);
    if mode >= STBDS_HM_STRING {
        let str_ptr = *(item_ptr as *mut *mut u8);
        libc::strcmp(key as *const i8, str_ptr as *const i8) == 0
    } else {
        libc::memcmp(key, item_ptr as *const c_void, keysize) == 0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    let table = stbds_hash_table(a);
    if !table.is_null() {
        if (*table).string.mode == STBDS_SH_STRDUP {
            let length = (*stbds_header(a)).length;
            let mut i = 1;
            while i < length {
                let p = *((a as *mut u8).add(elemsize * i) as *mut *mut c_void);
                stbds_free(p);
                i += 1;
            }
        }
        stbds_strreset(&mut (*table).string as *mut stbds_string_arena);
    }
    stbds_free((*stbds_header(a)).hash_table);
    stbds_free(stbds_header(a) as *mut c_void);
}

unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw_a = stbds_hash_to_arr(a, elemsize);
    let table = stbds_hash_table(raw_a);
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut u8, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let mut step = STBDS_BUCKET_LENGTH;

    if hash < 2 {
        hash = hash.wrapping_add(2);
    }

    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

        let start = pos & STBDS_BUCKET_MASK;
        for i in start..STBDS_BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i]) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }

        let limit = pos & STBDS_BUCKET_MASK;
        for i in 0..limit {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i]) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }

        pos = pos.wrapping_add(step);
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset = 0;
    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return stbds_arr_to_hash(a, elemsize);
    } else {
        let raw_a = stbds_hash_to_arr(a, elemsize);
        let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
        if table.is_null() {
            *temp = -1;
        } else {
            let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
            if slot < 0 {
                *temp = STBDS_INDEX_EMPTY;
            } else {
                let b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
                *temp = (*b).index[(slot as usize) & STBDS_BUCKET_MASK];
            }
        }
        return a;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temp: isize = 0;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp as *mut isize, mode);
    *stbds_temp(stbds_hash_to_arr(p, elemsize)) = temp;
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    mut a: *mut c_void,
    elemsize: usize,
) -> *mut c_void {
    if a.is_null() || (*stbds_header(stbds_hash_to_arr(a, elemsize))).length == 0 {
        let prev = if a.is_null() {
            ptr::null_mut()
        } else {
            stbds_hash_to_arr(a, elemsize)
        };
        a = stbds_arrgrowf(prev, elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        a = stbds_arr_to_hash(a, elemsize);
    }
    a
}

unsafe fn stbds_strdup(str_: *mut u8) -> *mut u8 {
    let len = libc::strlen(str_ as *const i8) + 1;
    let p = stbds_realloc(ptr::null_mut(), len) as *mut u8;
    libc::memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let keyoffset: usize = 0;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = stbds_arr_to_hash(a, elemsize);
    }

    let mut raw_a = a;
    a = stbds_hash_to_arr(a, elemsize);

    let mut table = (*stbds_header(a)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count * 2
        };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            stbds_free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT
            } else {
                STBDS_SH_NONE
            };
        }
        (*stbds_header(a)).hash_table = nt as *mut c_void;
        table = nt;
    }

    {
        let mut hash = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut u8, (*table).seed)
        } else {
            stbds_hash_bytes(key, keysize, (*table).seed)
        };
        let mut step = STBDS_BUCKET_LENGTH;
        let mut tombstone: isize = -1;

        if hash < 2 {
            hash = hash.wrapping_add(2);
        }

        let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        let final_pos: usize;
        'outer: loop {
            let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

            let start = pos & STBDS_BUCKET_MASK;
            for i in start..STBDS_BUCKET_LENGTH {
                if (*bucket).hash[i] == hash {
                    if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i]) {
                        *stbds_temp(a) = (*bucket).index[i];
                        if mode >= STBDS_HM_STRING {
                            let item_key_ptr = *((raw_a as *mut u8)
                                .add(elemsize.wrapping_mul((*bucket).index[i] as usize))
                                .add(keyoffset)
                                as *mut *mut u8);
                            *stbds_temp_key(a) = item_key_ptr;
                        }
                        return stbds_arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    final_pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'outer;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
            }

            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
                if (*bucket).hash[i] == hash {
                    if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i]) {
                        *stbds_temp(a) = (*bucket).index[i];
                        return stbds_arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    final_pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'outer;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
            }

            pos = pos.wrapping_add(step);
            step += STBDS_BUCKET_LENGTH;
            pos &= (*table).slot_count - 1;
        }

        // found_empty_slot:
        let mut pos = final_pos;
        if tombstone >= 0 {
            pos = tombstone as usize;
            (*table).tombstone_count -= 1;
        }
        (*table).used_count += 1;

        {
            let i = stbds_arrlen(a);
            if (i as usize) + 1 > stbds_arrcap(a) {
                a = stbds_arrgrowf(a, elemsize, 1, 0);
            }
            raw_a = stbds_arr_to_hash(a, elemsize);

            (*stbds_header(a)).length = (i + 1) as usize;
            let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
            (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
            *stbds_temp(a) = i - 1;

            let item_ptr = (a as *mut u8).add((elemsize as usize).wrapping_mul(i as usize));
            match (*table).string.mode {
                x if x == STBDS_SH_STRDUP => {
                    let dup = stbds_strdup(key as *mut u8);
                    *(item_ptr as *mut *mut u8) = dup;
                    *stbds_temp_key(a) = dup;
                }
                x if x == STBDS_SH_ARENA => {
                    let alloc = stbds_stralloc(&mut (*table).string, key as *mut u8);
                    *(item_ptr as *mut *mut u8) = alloc;
                    *stbds_temp_key(a) = alloc;
                }
                STBDS_SH_DEFAULT => {
                    *(item_ptr as *mut *mut u8) = key as *mut u8;
                    *stbds_temp_key(a) = key as *mut u8;
                }
                _ => {
                    libc::memcpy(item_ptr as *mut c_void, key, keysize);
                }
            }
        }
        let _ = raw_a;
        return stbds_arr_to_hash(a, elemsize);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    ptr::write_bytes(a as *mut u8, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    stbds_arr_to_hash(a, elemsize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> *mut c_void {
    if a.is_null() {
        return ptr::null_mut();
    }
    let raw_a = stbds_hash_to_arr(a, elemsize);
    let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
    *stbds_temp(raw_a) = 0;
    if table.is_null() {
        return a;
    }
    let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }
    let mut b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
    let mut i = (slot as usize) & STBDS_BUCKET_MASK;
    let old_index = (*b).index[i];
    let final_index = stbds_arrlen(raw_a) - 1 - 1;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    *stbds_temp(raw_a) = 1;
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let p = *((a as *mut u8).add(elemsize.wrapping_mul(old_index as usize))
            as *mut *mut c_void);
        stbds_free(p);
    }

    if old_index != final_index {
        libc::memmove(
            (a as *mut u8).add(elemsize.wrapping_mul(old_index as usize)) as *mut c_void,
            (a as *mut u8).add(elemsize.wrapping_mul(final_index as usize)) as *const c_void,
            elemsize,
        );

        if mode == STBDS_HM_STRING {
            let key_addr = *((a as *mut u8)
                .add(elemsize.wrapping_mul(old_index as usize))
                .add(keyoffset) as *mut *mut u8);
            slot = stbds_hm_find_slot(a, elemsize, key_addr as *mut c_void, keysize, keyoffset, mode);
        } else {
            let key_addr = (a as *mut u8)
                .add(elemsize.wrapping_mul(old_index as usize))
                .add(keyoffset);
            slot = stbds_hm_find_slot(a, elemsize, key_addr as *mut c_void, keysize, keyoffset, mode);
        }
        b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
        i = (slot as usize) & STBDS_BUCKET_MASK;
        (*b).index[i] = old_index;
    }
    (*stbds_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
        stbds_free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*stbds_header(raw_a)).hash_table =
            stbds_make_hash_index((*table).slot_count, table) as *mut c_void;
        stbds_free(table as *mut c_void);
    }

    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut u8,
) -> *mut u8 {
    let len = libc::strlen(str_ as *const i8) + 1;
    if len > (*a).remaining {
        let blocksize_initial = (*a).block;
        let blocksize: usize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize_initial >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            // Allocate single oversized block
            let alloc_size = std::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = stbds_realloc(ptr::null_mut(), alloc_size) as *mut stbds_string_block;
            libc::memmove(
                (&mut (*sb).storage) as *mut [u8; 8] as *mut c_void,
                str_ as *const c_void,
                len,
            );
            if !(*a).storage.is_null() {
                (*sb).next = (*(*a).storage).next;
                (*(*a).storage).next = sb;
            } else {
                (*sb).next = ptr::null_mut();
                (*a).storage = sb;
                (*a).remaining = 0;
            }
            return (&mut (*sb).storage) as *mut [u8; 8] as *mut u8;
        } else {
            let alloc_size = std::mem::size_of::<stbds_string_block>() - 8 + blocksize;
            let sb = stbds_realloc(ptr::null_mut(), alloc_size) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    let storage_ptr = (&mut (*(*a).storage).storage) as *mut [u8; 8] as *mut u8;
    let p = storage_ptr.add((*a).remaining - len);
    (*a).remaining -= len;
    libc::memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        stbds_free(x as *mut c_void);
        x = y;
    }
    ptr::write_bytes(a, 0, 1);
}

// Static buffer for strkey
static mut STRKEY_BUFFER: [u8; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut u8 {
    let buf_ptr = &raw mut STRKEY_BUFFER as *mut u8;
    libc::sprintf(buf_ptr as *mut i8, b"test_%d\0".as_ptr() as *const i8, n);
    buf_ptr
}

// arr_ins
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_ins(num: c_int) {
    let mut arr: *mut c_int = ptr::null_mut();
    let elemsize = std::mem::size_of::<c_int>();

    for i in 0..5 {
        // arrpush(arr,1); arrpush(arr,2); arrpush(arr,3); arrpush(arr,4);
        for v in 1..=4 {
            // arrmaybegrow(a, 1)
            if arr.is_null()
                || (*stbds_header(arr as *mut c_void)).length + 1
                    > (*stbds_header(arr as *mut c_void)).capacity
            {
                arr = stbds_arrgrowf(arr as *mut c_void, elemsize, 1, 0) as *mut c_int;
            }
            let len = (*stbds_header(arr as *mut c_void)).length;
            *arr.add(len) = v;
            (*stbds_header(arr as *mut c_void)).length += 1;
        }

        // stbds_arrins(arr, i, num) = arrinsn(arr, i, 1); arr[i] = num
        // arrinsn(a, i, n) = arraddn(a, n); memmove(&a[i+n], &a[i], sizeof*a*(length-n-i))
        // arraddn(a, n) = arraddnindex(a, n) = arrmaybegrow(a, n); length += n; return length-n
        if arr.is_null()
            || (*stbds_header(arr as *mut c_void)).length + 1
                > (*stbds_header(arr as *mut c_void)).capacity
        {
            arr = stbds_arrgrowf(arr as *mut c_void, elemsize, 1, 0) as *mut c_int;
        }
        (*stbds_header(arr as *mut c_void)).length += 1;
        let length = (*stbds_header(arr as *mut c_void)).length;
        // memmove(&arr[i+1], &arr[i], sizeof*arr * (length - 1 - i))
        let n_move = length.wrapping_sub(1).wrapping_sub(i as usize);
        libc::memmove(
            arr.add(i as usize + 1) as *mut c_void,
            arr.add(i as usize) as *const c_void,
            elemsize * n_move,
        );
        *arr.add(i as usize) = num;

        // assertions
        assert!(*arr.add(i as usize) == num);
        if i < 4 {
            assert!(*arr.add(4) == 4);
        }

        // arrfree
        if !arr.is_null() {
            stbds_free(stbds_header(arr as *mut c_void) as *mut c_void);
        }
        arr = ptr::null_mut();
    }
    let _ = arr;
}
