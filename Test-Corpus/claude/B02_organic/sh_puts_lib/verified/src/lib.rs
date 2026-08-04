// Rust translation of c_src/src/lib.c
//
// This translation re-implements the subset of the stb_ds data-structures
// library used by lib.c, exporting the same `extern "C"` symbols so that the
// resulting cdylib is a drop-in replacement for the C shared library.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::c_void;
use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// libc bindings
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// Public top-level function
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_puts(num: c_int) {
    // Mirror the C implementation's externally visible behavior. After all
    // the (purely heap-side) stbds bookkeeping, the C function prints the
    // single map entry which has key "a" and value `num`.

    // Step 1: stralloc / strreset on a fresh arena. These have no observable
    // side effects beyond heap allocations, but we replicate them so any
    // shared mutable state (the static `buffer` used by `strkey`) ends up in
    // the same final state as the C implementation.
    let mut sa: stbds_string_arena = core::mem::zeroed();
    for i in 0..num {
        let k = strkey(i);
        stbds_stralloc(&mut sa, k);
    }
    stbds_strreset(&mut sa);

    // Step 2: build a single-element string-arena map and print it.
    #[repr(C)]
    struct StrMap {
        key: *mut c_char,
        value: c_int,
    }
    let elemsize = core::mem::size_of::<StrMap>();
    // sh_new_arena: sets up a string-keyed map in arena mode.
    let mut strmap = stbds_shmode_func(elemsize, STBDS_SH_ARENA) as *mut StrMap;
    let key_a = b"a\0".as_ptr() as *mut c_char;
    // shputs(strmap, s) where s = { key: "a", value: num }
    // Translates to: stbds_hmput_key with mode STBDS_HM_STRING, then assign
    // the entire struct, then re-read the duplicated key pointer.
    {
        let s = StrMap { key: key_a, value: num };
        // For STRING mode, hmput_key takes the char* itself, not &char*.
        let key_ptr = s.key as *mut c_void;
        let key_size = core::mem::size_of::<*mut c_char>();
        strmap = stbds_hmput_key(
            strmap as *mut c_void,
            elemsize,
            key_ptr,
            key_size,
            STBDS_HM_STRING,
        ) as *mut StrMap;
        let raw = (strmap as *mut u8).sub(elemsize) as *mut StrMap;
        let temp_idx = (*stbds_header(raw as *mut c_void)).temp;
        let target = strmap.offset(temp_idx);
        let temp_key_after =
            (*((*stbds_header(raw as *mut c_void)).hash_table as *mut stbds_hash_index)).temp_key;
        (*target) = StrMap { key: s.key, value: s.value };
        // After hmput in string mode, the table's temp_key points to the
        // (possibly duplicated) key string. Use it as the slot's key.
        (*target).key = temp_key_after;
    }

    // Step 3: print map contents, then free.
    let len = stbds_hmlen(strmap as *mut c_void, elemsize);
    let mut z: isize = 0;
    while z < len {
        let entry = strmap.offset(z);
        // printf("%s %d\n", strmap[z], strmap[z].value);
        // The struct is passed by value through varargs. Per System V AMD64
        // ABI, a struct of {char*, int} (16 bytes total) is passed in two
        // 8-byte registers; printf consumes them via %s and %d.
        printf(
            b"%s %d\n\0".as_ptr() as *const c_char,
            (*entry).key,
            (*entry).value,
        );
        z += 1;
    }
    // shfree(strmap)
    stbds_hmfree_func(
        (strmap as *mut u8).sub(elemsize) as *mut c_void,
        elemsize,
    );
}

// ---------------------------------------------------------------------------
// strkey - sprintf-based test helper exported by the C library.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub static mut buffer: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    sprintf(
        buffer.as_mut_ptr(),
        b"test_%d\0".as_ptr() as *const c_char,
        n,
    );
    buffer.as_mut_ptr()
}

// ---------------------------------------------------------------------------
// stb_ds data structures (matching the C layout exactly)
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct stbds_array_header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut c_void,
    pub temp: isize,
}

#[repr(C)]
pub struct stbds_string_block {
    pub next: *mut stbds_string_block,
    pub storage: [c_char; 8],
}

#[repr(C)]
pub struct stbds_string_arena {
    pub storage: *mut stbds_string_block,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

#[repr(C)]
pub struct stbds_hash_bucket {
    pub hash: [usize; STBDS_BUCKET_LENGTH],
    pub index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
pub struct stbds_hash_index {
    pub temp_key: *mut c_char,
    pub slot_count: usize,
    pub used_count: usize,
    pub used_count_threshold: usize,
    pub used_count_shrink_threshold: usize,
    pub tombstone_count: usize,
    pub tombstone_count_threshold: usize,
    pub seed: usize,
    pub slot_count_log2: usize,
    pub string: stbds_string_arena,
    pub storage: *mut stbds_hash_bucket,
}

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: c_int = 0;
const STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;

#[inline]
unsafe fn stbds_header(t: *mut c_void) -> *mut stbds_array_header {
    if t.is_null() {
        core::ptr::null_mut()
    } else {
        (t as *mut stbds_array_header).sub(1)
    }
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
unsafe fn stbds_index_in_use(x: isize) -> bool {
    x >= 0
}

#[inline]
unsafe fn stbds_align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

#[inline]
unsafe fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).sub(elemsize) as *mut c_void
}

#[inline]
unsafe fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    (x as *mut u8).add(elemsize) as *mut c_void
}

#[inline]
unsafe fn hash_table_of(a: *mut c_void) -> *mut stbds_hash_index {
    (*stbds_header(a)).hash_table as *mut stbds_hash_index
}

// ---------------------------------------------------------------------------
// Random seed
// ---------------------------------------------------------------------------

static mut STBDS_HASH_SEED: usize = 0x31415926;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED = seed;
}

// ---------------------------------------------------------------------------
// Hash functions
// ---------------------------------------------------------------------------

const STBDS_SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() * 8) as u32;

#[inline(always)]
fn rotl(val: usize, n: u32) -> usize {
    (val << n) | (val >> (STBDS_SIZE_T_BITS - n))
}

#[inline(always)]
fn rotr(val: usize, n: u32) -> usize {
    (val >> n) | (val << (STBDS_SIZE_T_BITS - n))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut str: *mut c_char, seed: usize) -> usize {
    let mut hash: usize = seed;
    while *str != 0 {
        hash = rotl(hash, 9).wrapping_add(*(str as *mut u8) as usize);
        str = str.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ rotr(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ rotr(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= rotr(hash, 22);
    hash.wrapping_add(seed)
}

#[inline(always)]
fn siphash_round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotl(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotl(*v0, STBDS_SIZE_T_BITS / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotl(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotl(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotl(*v2, STBDS_SIZE_T_BITS / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotl(*v3, 21);
    *v3 ^= *v0;
}

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

unsafe fn stbds_siphash_bytes(p: *const c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;

    // (((size_t) 0x736f6d65 << 16) << 16) + 0x70736575
    let mut v0: usize = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1: usize = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: usize = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3: usize = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let elem = core::mem::size_of::<usize>();
    let mut i: usize = 0;
    let mut data: usize;
    while i + elem <= len {
        // C: data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24)
        //   each d[i] promotes to int; the whole expression is int.
        //   Assigning int to size_t sign-extends if the int is negative
        //   (i.e., if d[3] >= 0x80, the high half of size_t becomes 1s).
        let b0 = *d as i32;
        let b1 = *d.add(1) as i32;
        let b2 = *d.add(2) as i32;
        let b3 = *d.add(3) as i32;
        let lo_int: i32 = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
        // Sign-extend to size_t.
        let lo_size = lo_int as i64 as usize;
        // C: data |= (size_t)(d[4] | (d[5] << 8) | (d[6] << 16) | (d[7] << 24)) << 16 << 16
        //   The cast is to size_t (unsigned), so bytes 4-7 are zero-extended.
        let b4 = *d.add(4) as u32;
        let b5 = *d.add(5) as u32;
        let b6 = *d.add(6) as u32;
        let b7 = *d.add(7) as u32;
        let hi32 = b4 | (b5 << 8) | (b6 << 16) | (b7 << 24);
        let hi: usize = ((hi32 as usize) << 16) << 16;
        data = lo_size | hi;

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;
        i += elem;
        d = d.add(elem);
    }

    data = len << (STBDS_SIZE_T_BITS - 8);
    let tail = len - i;
    // Replicate C fall-through switch with C's integer-promotion semantics.
    if tail >= 7 {
        data |= (*d.add(6) as usize) << 24 << 24;
    }
    if tail >= 6 {
        data |= (*d.add(5) as usize) << 20 << 20;
    }
    if tail >= 5 {
        data |= (*d.add(4) as usize) << 16 << 16;
    }
    if tail >= 4 {
        // C: `(d[3] << 24)` - d[3] promoted to int. If d[3] >= 0x80, the
        // result has bit 31 set and is sign-extended to size_t.
        let v = (*d.add(3) as i32) << 24;
        data |= v as i64 as usize;
    }
    if tail >= 3 {
        data |= (*d.add(2) as usize) << 16;
    }
    if tail >= 2 {
        data |= (*d.add(1) as usize) << 8;
    }
    if tail >= 1 {
        data |= *d as usize;
    }

    v3 ^= data;
    for _ in 0..STBDS_SIPHASH_C_ROUNDS {
        siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..STBDS_SIPHASH_D_ROUNDS {
        siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

// ---------------------------------------------------------------------------
// Array growth / free
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = stbds_arrlen(a) as usize + addlen;
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

    let header_size = core::mem::size_of::<stbds_array_header>();
    let bytes = elemsize * min_cap + header_size;
    let old_header = if a.is_null() {
        core::ptr::null_mut()
    } else {
        stbds_header(a) as *mut c_void
    };
    let mut b = realloc(old_header, bytes);
    b = (b as *mut u8).add(header_size) as *mut c_void;
    if a.is_null() {
        let h = stbds_header(b);
        (*h).length = 0;
        (*h).hash_table = core::ptr::null_mut();
        (*h).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(stbds_header(a) as *mut c_void);
}

// ---------------------------------------------------------------------------
// Hash index management
// ---------------------------------------------------------------------------

unsafe fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n = 0usize;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

unsafe fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

unsafe fn stbds_make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    let bytes = (slot_count >> STBDS_BUCKET_SHIFT) * core::mem::size_of::<stbds_hash_bucket>()
        + core::mem::size_of::<stbds_hash_index>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let t = realloc(core::ptr::null_mut(), bytes) as *mut stbds_hash_index;
    let storage_addr = stbds_align_fwd(t.add(1) as usize, STBDS_CACHE_LINE_SIZE);
    (*t).storage = storage_addr as *mut stbds_hash_bucket;
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
        (*t).string = stbds_string_arena {
            storage: (*ot).string.storage,
            remaining: (*ot).string.remaining,
            block: (*ot).string.block,
            mode: (*ot).string.mode,
        };
        (*t).seed = (*ot).seed;
    } else {
        (*t).string = stbds_string_arena {
            storage: core::ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        };
        (*t).seed = STBDS_HASH_SEED;
        // stbds_load_32_or_64(a, temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd):
        //   on a 64-bit platform a = (0x27bb2ee6 << 32) ^ 0x87b0b0fd ^ 2147001325 ^ 2147001325
        //   simplifies to 0x27bb2ee687b0b0fdusize
        let a: usize = 0x27bb2ee687b0b0fd_usize;
        // stbds_load_32_or_64(b, temp, 715136305, 0, 0xb504f32d):
        //   b = 0xb504f32dusize (since high half is 0)
        let b: usize = 0xb504f32d_usize;
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    let buckets = slot_count >> STBDS_BUCKET_SHIFT;
    for i in 0..buckets {
        let bucket = (*t).storage.add(i);
        for j in 0..STBDS_BUCKET_LENGTH {
            (*bucket).hash[j] = STBDS_HASH_EMPTY;
        }
        for j in 0..STBDS_BUCKET_LENGTH {
            (*bucket).index[j] = STBDS_INDEX_EMPTY;
        }
    }

    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let ot_buckets = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..ot_buckets {
            let ob = (*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if stbds_index_in_use((*ob).index[j]) {
                    let hash = (*ob).hash[j];
                    let mut pos =
                        stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'outer: loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);
                        let start = pos & STBDS_BUCKET_MASK;
                        for z in start..STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'outer;
                            }
                        }
                        for z in 0..start {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'outer;
                            }
                        }
                        pos += step;
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
    let entry = (a as *mut u8).add(elemsize.wrapping_mul(i as usize)).add(keyoffset);
    if mode >= STBDS_HM_STRING {
        // The slot holds a (char*) at this offset.
        let stored = *(entry as *mut *mut c_char);
        strcmp(key as *mut c_char, stored) == 0
    } else {
        memcmp(key, entry as *const c_void, keysize) == 0
    }
}

unsafe fn stbds_hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    let raw_a = hash_to_arr(a, elemsize);
    let table = hash_table_of(raw_a);
    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    if hash < 2 {
        hash += 2;
    }
    let mut step = STBDS_BUCKET_LENGTH;
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
        for i in 0..start {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i]) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }
        pos += step;
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    let table = hash_table_of(a);
    if !table.is_null() {
        if (*table).string.mode == STBDS_SH_STRDUP as u8 {
            let length = (*stbds_header(a)).length;
            for i in 1..length {
                let slot = (a as *mut u8).add(elemsize * i) as *mut *mut c_char;
                free(*slot as *mut c_void);
            }
        }
        stbds_strreset(&mut (*table).string);
    }
    free((*stbds_header(a)).hash_table);
    free(stbds_header(a) as *mut c_void);
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
    let keyoffset = 0usize;
    if a.is_null() {
        a = stbds_arrgrowf(core::ptr::null_mut(), elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        memset(a, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        return arr_to_hash(a, elemsize);
    }
    let raw_a = hash_to_arr(a, elemsize);
    let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
    if table.is_null() {
        *temp = -1;
    } else {
        let slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
        if slot < 0 {
            *temp = STBDS_INDEX_EMPTY;
        } else {
            let b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
            *temp = (*b).index[slot as usize & STBDS_BUCKET_MASK];
        }
    }
    a
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
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    (*stbds_header(hash_to_arr(p, elemsize))).temp = temp;
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(
    mut a: *mut c_void,
    elemsize: usize,
) -> *mut c_void {
    if a.is_null() || (*stbds_header(hash_to_arr(a, elemsize))).length == 0 {
        let in_a = if a.is_null() {
            core::ptr::null_mut()
        } else {
            hash_to_arr(a, elemsize)
        };
        a = stbds_arrgrowf(in_a, elemsize, 0, 1);
        (*stbds_header(a)).length += 1;
        memset(a, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a
}

unsafe fn stbds_strdup(s: *mut c_char) -> *mut c_char {
    let len = strlen(s) + 1;
    let p = realloc(core::ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, s as *const c_void, len);
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
    let keyoffset = 0usize;

    if a.is_null() {
        a = stbds_arrgrowf(core::ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }

    let mut raw_a = a;
    let mut a_inner = hash_to_arr(a, elemsize);

    let mut table = (*stbds_header(a_inner)).hash_table as *mut stbds_hash_index;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() {
            STBDS_BUCKET_LENGTH
        } else {
            (*table).slot_count * 2
        };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            free(table as *mut c_void);
        } else {
            (*nt).string.mode = if mode >= STBDS_HM_STRING {
                STBDS_SH_DEFAULT as u8
            } else {
                0
            };
        }
        (*stbds_header(a_inner)).hash_table = nt as *mut c_void;
        table = nt;
    }

    let mut hash = if mode >= STBDS_HM_STRING {
        stbds_hash_string(key as *mut c_char, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    if hash < 2 {
        hash += 2;
    }

    let mut step = STBDS_BUCKET_LENGTH;
    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);
    let mut tombstone: isize = -1;

    let pos_final;

    'outer: loop {
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        let start = pos & STBDS_BUCKET_MASK;
        for i in start..STBDS_BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i]) {
                    (*stbds_header(a_inner)).temp = (*bucket).index[i];
                    if mode >= STBDS_HM_STRING {
                        let stored = *((raw_a as *mut u8)
                            .add(elemsize * (*bucket).index[i] as usize)
                            .add(keyoffset)
                            as *mut *mut c_char);
                        (*((*stbds_header(a_inner)).hash_table as *mut stbds_hash_index)).temp_key =
                            stored;
                    }
                    return arr_to_hash(a_inner, elemsize);
                }
            } else if (*bucket).hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                pos_final = pos;
                break 'outer;
            } else if tombstone < 0
                && (*bucket).index[i] == STBDS_INDEX_DELETED
            {
                tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
            }
        }
        for i in 0..start {
            if (*bucket).hash[i] == hash {
                if stbds_is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i]) {
                    (*stbds_header(a_inner)).temp = (*bucket).index[i];
                    return arr_to_hash(a_inner, elemsize);
                }
            } else if (*bucket).hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                pos_final = pos;
                break 'outer;
            } else if tombstone < 0
                && (*bucket).index[i] == STBDS_INDEX_DELETED
            {
                tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
            }
        }
        pos += step;
        step += STBDS_BUCKET_LENGTH;
        pos &= (*table).slot_count - 1;
    }

    let mut pos = pos_final;
    if tombstone >= 0 {
        pos = tombstone as usize;
        (*table).tombstone_count -= 1;
    }
    (*table).used_count += 1;

    let i = stbds_arrlen(a_inner);
    if (i as usize + 1) > stbds_arrcap(a_inner) {
        a_inner = stbds_arrgrowf(a_inner, elemsize, 1, 0);
    }
    raw_a = arr_to_hash(a_inner, elemsize);

    (*stbds_header(a_inner)).length = (i + 1) as usize;
    let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
    (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
    (*stbds_header(a_inner)).temp = i - 1;

    let slot_ptr = (a_inner as *mut u8).add(elemsize * i as usize) as *mut *mut c_char;
    match (*table).string.mode {
        m if m == STBDS_SH_STRDUP as u8 => {
            let dup = stbds_strdup(key as *mut c_char);
            *slot_ptr = dup;
            (*table).temp_key = dup;
        }
        m if m == STBDS_SH_ARENA as u8 => {
            let arena_ptr = stbds_stralloc(&mut (*table).string, key as *mut c_char);
            *slot_ptr = arena_ptr;
            (*table).temp_key = arena_ptr;
        }
        m if m == STBDS_SH_DEFAULT as u8 => {
            *slot_ptr = key as *mut c_char;
            (*table).temp_key = key as *mut c_char;
        }
        _ => {
            memcpy(
                (a_inner as *mut u8).add(elemsize * i as usize) as *mut c_void,
                key,
                keysize,
            );
        }
    }

    arr_to_hash(a_inner, elemsize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(core::ptr::null_mut(), elemsize, 0, 1);
    memset(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, core::ptr::null_mut());
    (*stbds_header(a)).hash_table = h as *mut c_void;
    (*h).string.mode = mode as u8;
    arr_to_hash(a, elemsize)
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
        return core::ptr::null_mut();
    }
    let raw_a = hash_to_arr(a, elemsize);
    let table = (*stbds_header(raw_a)).hash_table as *mut stbds_hash_index;
    (*stbds_header(raw_a)).temp = 0;
    if table.is_null() {
        return a;
    }
    let mut slot = stbds_hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
    if slot < 0 {
        return a;
    }

    let mut b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
    let mut i = slot as usize & STBDS_BUCKET_MASK;
    let old_index = (*b).index[i];
    let final_index = stbds_arrlen(raw_a) - 1 - 1;
    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*stbds_header(raw_a)).temp = 1;
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP as u8 {
        let p = (a as *mut u8).add(elemsize * old_index as usize) as *mut *mut c_char;
        free(*p as *mut c_void);
    }

    if old_index != final_index {
        memmove(
            (a as *mut u8).add(elemsize * old_index as usize) as *mut c_void,
            (a as *mut u8).add(elemsize * final_index as usize) as *mut c_void,
            elemsize,
        );

        let new_slot = if mode == STBDS_HM_STRING {
            let stored = *((a as *mut u8)
                .add(elemsize * old_index as usize)
                .add(keyoffset)
                as *mut *mut c_char);
            stbds_hm_find_slot(a, elemsize, stored as *mut c_void, keysize, keyoffset, mode)
        } else {
            let key_ptr = (a as *mut u8)
                .add(elemsize * old_index as usize)
                .add(keyoffset);
            stbds_hm_find_slot(a, elemsize, key_ptr as *mut c_void, keysize, keyoffset, mode)
        };
        slot = new_slot;
        b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
        i = slot as usize & STBDS_BUCKET_MASK;
        (*b).index[i] = old_index;
    }
    (*stbds_header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold
        && (*table).slot_count > STBDS_BUCKET_LENGTH
    {
        let nt = stbds_make_hash_index((*table).slot_count >> 1, table);
        (*stbds_header(raw_a)).hash_table = nt as *mut c_void;
        free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        let nt = stbds_make_hash_index((*table).slot_count, table);
        (*stbds_header(raw_a)).hash_table = nt as *mut c_void;
        free(table as *mut c_void);
    }

    a
}

// ---------------------------------------------------------------------------
// String arena
// ---------------------------------------------------------------------------

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    s: *mut c_char,
) -> *mut c_char {
    let len = strlen(s) + 1;
    if len > (*a).remaining {
        let blocksize_pre = (*a).block as usize;
        let blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize_pre >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            // Allocate exactly len-bytes block.
            let alloc_size = core::mem::size_of::<stbds_string_block>() - 8 + len;
            let sb = realloc(core::ptr::null_mut(), alloc_size) as *mut stbds_string_block;
            memmove((*sb).storage.as_mut_ptr() as *mut c_void, s as *const c_void, len);
            if !(*a).storage.is_null() {
                (*sb).next = (*(*a).storage).next;
                (*(*a).storage).next = sb;
            } else {
                (*sb).next = core::ptr::null_mut();
                (*a).storage = sb;
                (*a).remaining = 0;
            }
            return (*sb).storage.as_mut_ptr();
        } else {
            let alloc_size = core::mem::size_of::<stbds_string_block>() - 8 + blocksize;
            let sb = realloc(core::ptr::null_mut(), alloc_size) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    let p = (*(*a).storage).storage.as_mut_ptr().add((*a).remaining - len);
    (*a).remaining -= len;
    memmove(p as *mut c_void, s as *const c_void, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        free(x as *mut c_void);
        x = y;
    }
    memset(
        a as *mut c_void,
        0,
        core::mem::size_of::<stbds_string_arena>(),
    );
}

// ---------------------------------------------------------------------------
// Helpers used by sh_puts that aren't exported
// ---------------------------------------------------------------------------

unsafe fn stbds_hmlen(t: *mut c_void, elemsize: usize) -> isize {
    if t.is_null() {
        0
    } else {
        let raw = (t as *mut u8).sub(elemsize) as *mut c_void;
        (*stbds_header(raw)).length as isize - 1
    }
}

// Suppress dead-code warnings for constants we don't reference in this build.
#[allow(dead_code)]
const _UNUSED_CONSTS: &[c_int] = &[STBDS_HM_BINARY, STBDS_SH_NONE];
