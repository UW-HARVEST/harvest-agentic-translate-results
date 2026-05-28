// Rust translation of c_src/src/lib.c
//
// The C library exposes the public function `arr_ins(int num)` declared in
// c_src/include/lib.h. The implementation in lib.c includes a complete
// embedded copy of stb_ds (dynamic arrays + hash maps) plus a small `strkey`
// helper. Even though `arr_ins` itself only uses the dynamic array macros,
// the C .so still exports every non-static function defined in lib.c. To
// keep the Rust .so's exported-symbol set identical to the C one, we
// translate each of those functions verbatim.
//
// The functions operate on raw memory layouts that downstream callers depend
// on byte-for-byte (header is placed *before* the user-visible array
// pointer, hash buckets are aligned to 64-byte cache lines, etc.). The Rust
// translation therefore uses raw pointer arithmetic and `libc::realloc /
// libc::free` so that pointers can flow between the C .so and the Rust .so
// interchangeably if a caller ever wished. We deliberately mirror the C
// control flow line-for-line.

#![allow(non_camel_case_types)]
#![allow(private_interfaces)]
#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(dest: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
}

// --- Constants from lib.c ---

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3; // STBDS_BUCKET_LENGTH == 8 -> 3
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_HM_BINARY: c_int = 0;
const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_NONE: c_int = 0;
const _STBDS_SH_DEFAULT: c_int = 1;
const STBDS_SH_STRDUP: c_int = 2;
const STBDS_SH_ARENA: c_int = 3;
// Used by stbds_hmput_key when promoting the freshly-allocated table out of
// SH_NONE if the caller chose a string-keyed mode. The C source also uses
// this enum value.
const STBDS_SH_DEFAULT_FOR_STRING: c_int = 1;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_SIPHASH_C_ROUNDS: usize = 2;
const STBDS_SIPHASH_D_ROUNDS: usize = 4;

const STBDS_SIZE_T_BITS: u32 = (size_of::<usize>() as u32) * 8;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

// --- Layout structures (must match C exactly) ---

#[repr(C)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [c_char; 8],
}

#[repr(C)]
struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
struct stbds_hash_bucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
struct stbds_hash_index {
    temp_key: *mut c_char,
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

// --- Global seed (matches `static size_t stbds_hash_seed = 0x31415926;`) ---

static mut STBDS_HASH_SEED: usize = 0x31415926;

// --- Helpers (the C "stbds_header" macro) ---

#[inline]
unsafe fn header(a: *mut c_void) -> *mut stbds_array_header {
    unsafe { (a as *mut stbds_array_header).offset(-1) }
}

#[inline]
unsafe fn arrlen(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        unsafe { (*header(a)).length as isize }
    }
}

#[inline]
unsafe fn arrcap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*header(a)).capacity }
    }
}

#[inline]
fn align_fwd(n: usize, a: usize) -> usize {
    (n + (a - 1)) & !(a - 1)
}

// --- stbds_rand_seed ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe {
        STBDS_HASH_SEED = seed;
    }
}

// --- stbds_arrgrowf ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    unsafe {
        let min_len = (arrlen(a) as usize).wrapping_add(addlen);
        if min_len > min_cap {
            min_cap = min_len;
        }
        if min_cap <= arrcap(a) {
            return a;
        }

        if min_cap < 2 * arrcap(a) {
            min_cap = 2 * arrcap(a);
        } else if min_cap < 4 {
            min_cap = 4;
        }

        let prev: *mut c_void = if a.is_null() {
            ptr::null_mut()
        } else {
            header(a) as *mut c_void
        };
        let total = elemsize * min_cap + size_of::<stbds_array_header>();
        let mut b = realloc(prev, total);
        b = (b as *mut u8).add(size_of::<stbds_array_header>()) as *mut c_void;

        if a.is_null() {
            (*header(b)).length = 0;
            (*header(b)).hash_table = ptr::null_mut();
            (*header(b)).temp = 0;
        }
        (*header(b)).capacity = min_cap;
        b
    }
}

// --- stbds_arrfreef ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    unsafe {
        free(header(a) as *mut c_void);
    }
}

// --- stbds_hash_string ---

#[inline]
fn rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline]
fn rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut str_: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut hash: usize = seed;
        while *str_ != 0 {
            hash = rotate_left(hash, 9).wrapping_add(*(str_ as *const u8) as usize);
            str_ = str_.add(1);
        }

        hash ^= seed;
        // hash = (~hash) + (hash << 18)
        hash = (!hash).wrapping_add(hash << 18);
        // hash ^= hash ^ ROTATE_RIGHT(hash, 31)  ==>  hash = ROTATE_RIGHT(hash, 31)
        hash ^= hash ^ rotate_right(hash, 31);
        hash = hash.wrapping_mul(21);
        hash ^= hash ^ rotate_right(hash, 11);
        hash = hash.wrapping_add(hash << 6);
        hash ^= rotate_right(hash, 22);
        hash.wrapping_add(seed)
    }
}

// --- stbds_siphash_bytes / stbds_hash_bytes ---

#[inline]
unsafe fn siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe {
        let mut d = p as *mut u8;
        let not_seed = !seed;

        let mut v0: usize = (((0x736f6d65usize) << 16) << 16).wrapping_add(0x70736575) ^ seed;
        let mut v1: usize = (((0x646f7261usize) << 16) << 16).wrapping_add(0x6e646f6d) ^ not_seed;
        let mut v2: usize = (((0x6c796765usize) << 16) << 16).wrapping_add(0x6e657261) ^ seed;
        let mut v3: usize = (((0x74656462usize) << 16) << 16).wrapping_add(0x79746573) ^ not_seed;

        v0 ^= 0x0706050403020100usize ^ seed;
        v1 ^= 0x0f0e0d0c0b0a0908usize ^ not_seed;
        v2 ^= 0x0706050403020100usize ^ seed;
        v3 ^= 0x0f0e0d0c0b0a0908usize ^ not_seed;

        macro_rules! sipround {
            () => {{
                v0 = v0.wrapping_add(v1);
                v1 = rotate_left(v1, 13);
                v1 ^= v0;
                v0 = rotate_left(v0, STBDS_SIZE_T_BITS / 2);
                v2 = v2.wrapping_add(v3);
                v3 = rotate_left(v3, 16);
                v3 ^= v2;
                v2 = v2.wrapping_add(v1);
                v1 = rotate_left(v1, 17);
                v1 ^= v2;
                v2 = rotate_left(v2, STBDS_SIZE_T_BITS / 2);
                v0 = v0.wrapping_add(v3);
                v3 = rotate_left(v3, 21);
                v3 ^= v0;
            }};
        }

        let mut i: usize = 0;
        while i + size_of::<usize>() <= len {
            // data = d[0] | (d[1]<<8) | (d[2]<<16) | (d[3]<<24);
            // data |= ((d[4]|(d[5]<<8)|(d[6]<<16)|(d[7]<<24))) << 16 << 16;
            let mut data: usize = (*d.add(0)) as usize
                | ((*d.add(1) as usize) << 8)
                | ((*d.add(2) as usize) << 16)
                | ((*d.add(3) as usize) << 24);
            let high: usize = (*d.add(4) as usize)
                | ((*d.add(5) as usize) << 8)
                | ((*d.add(6) as usize) << 16)
                | ((*d.add(7) as usize) << 24);
            data |= (high << 16) << 16;

            v3 ^= data;
            for _ in 0..STBDS_SIPHASH_C_ROUNDS {
                sipround!();
            }
            v0 ^= data;

            i += size_of::<usize>();
            d = d.add(size_of::<usize>());
        }

        let mut data: usize = len << (STBDS_SIZE_T_BITS - 8);
        let rem = len - i;
        // C uses fall-through; replicate that explicitly.
        // d at this point points at the leftover bytes.
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
            data |= (*d.add(3) as usize) << 24;
        }
        if rem >= 3 {
            data |= (*d.add(2) as usize) << 16;
        }
        if rem >= 2 {
            data |= (*d.add(1) as usize) << 8;
        }
        if rem >= 1 {
            data |= *d.add(0) as usize;
        }

        v3 ^= data;
        for _ in 0..STBDS_SIPHASH_C_ROUNDS {
            sipround!();
        }
        v0 ^= data;
        v2 ^= 0xff;
        for _ in 0..STBDS_SIPHASH_D_ROUNDS {
            sipround!();
        }

        v0 ^ v1 ^ v2 ^ v3
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { siphash_bytes(p, len, seed) }
}

// --- internal: stbds_log2, stbds_probe_position, stbds_make_hash_index ---

fn log2_bits(mut slot_count: usize) -> usize {
    let mut n = 0usize;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

#[inline]
fn probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

unsafe fn make_hash_index(
    slot_count: usize,
    ot: *mut stbds_hash_index,
) -> *mut stbds_hash_index {
    unsafe {
        let bytes = (slot_count >> STBDS_BUCKET_SHIFT) * size_of::<stbds_hash_bucket>()
            + size_of::<stbds_hash_index>()
            + STBDS_CACHE_LINE_SIZE
            - 1;
        let t = realloc(ptr::null_mut(), bytes) as *mut stbds_hash_index;
        let after = (t.add(1)) as usize;
        (*t).storage = align_fwd(after, STBDS_CACHE_LINE_SIZE) as *mut stbds_hash_bucket;
        (*t).slot_count = slot_count;
        (*t).slot_count_log2 = log2_bits(slot_count);
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
            memset(
                &mut (*t).string as *mut _ as *mut c_void,
                0,
                size_of::<stbds_string_arena>(),
            );
            (*t).seed = STBDS_HASH_SEED;
            // stbds_load_32_or_64(a, temp, 2147001325, 0x27bb2ee6, 0x87b0b0fd);
            // For 64-bit: a = 0x27bb2ee687b0b0fd
            // For 32-bit: a = 2147001325
            #[cfg(target_pointer_width = "64")]
            let a: usize = (0x27bb2ee6usize << 32) | 0x87b0b0fd;
            #[cfg(target_pointer_width = "32")]
            let a: usize = 2147001325;
            // stbds_load_32_or_64(b, temp, 715136305, 0, 0xb504f32d);
            #[cfg(target_pointer_width = "64")]
            let b: usize = (0usize << 32) | 0xb504f32d;
            #[cfg(target_pointer_width = "32")]
            let b: usize = 715136305;
            STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
        }

        // Initialize buckets.
        let nbuckets = slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..nbuckets {
            let b_ptr = (*t).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b_ptr).hash[j] = STBDS_HASH_EMPTY;
            }
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b_ptr).index[j] = STBDS_INDEX_EMPTY;
            }
        }

        if !ot.is_null() {
            (*t).used_count = (*ot).used_count;
            let nold = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
            for i in 0..nold {
                let ob = (*ot).storage.add(i);
                for j in 0..STBDS_BUCKET_LENGTH {
                    if (*ob).index[j] >= 0 {
                        let hash = (*ob).hash[j];
                        let mut pos = probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
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

                            let limit = pos & STBDS_BUCKET_MASK;
                            let mut placed = false;
                            for z in 0..limit {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = (*ob).index[j];
                                    placed = true;
                                    break;
                                }
                            }
                            if placed {
                                break 'outer;
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
}

// --- helpers for hash table threading on the array header ---

#[inline]
unsafe fn hash_to_arr(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).sub(elemsize) as *mut c_void }
}
#[inline]
unsafe fn arr_to_hash(x: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (x as *mut u8).add(elemsize) as *mut c_void }
}
#[inline]
unsafe fn hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*header(a)).hash_table as *mut stbds_hash_index }
}
#[inline]
unsafe fn temp_of(a: *mut c_void) -> *mut isize {
    unsafe { &mut (*header(a)).temp as *mut isize }
}
#[inline]
unsafe fn temp_key_of(a: *mut c_void) -> *mut *mut c_char {
    unsafe { &mut (*header(a)).hash_table as *mut *mut c_void as *mut *mut c_char }
}

// --- stbds_hmfree_func ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    unsafe {
        if a.is_null() {
            return;
        }
        let table = hash_table(a);
        if !table.is_null() {
            if (*table).string.mode as c_int == STBDS_SH_STRDUP {
                let len = (*header(a)).length;
                for i in 1..len {
                    let pp = (a as *mut u8).add(elemsize * i) as *mut *mut c_char;
                    free(*pp as *mut c_void);
                }
            }
            stbds_strreset(&mut (*table).string as *mut stbds_string_arena);
        }
        free((*header(a)).hash_table);
        free(header(a) as *mut c_void);
    }
}

// --- stbds_hm_find_slot (internal) ---

#[inline]
unsafe fn is_key_equal(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
    i: usize,
) -> bool {
    unsafe {
        if mode >= STBDS_HM_STRING {
            let stored = *((a as *mut u8).add(elemsize * i + keyoffset) as *mut *mut c_char);
            strcmp(key as *const c_char, stored) == 0
        } else {
            memcmp(
                key,
                (a as *mut u8).add(elemsize * i + keyoffset) as *const c_void,
                keysize,
            ) == 0
        }
    }
}

unsafe fn hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
    unsafe {
        let raw_a = hash_to_arr(a, elemsize);
        let table = hash_table(raw_a);
        let mut hash = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut c_char, (*table).seed)
        } else {
            stbds_hash_bytes(key, keysize, (*table).seed)
        };
        if hash < 2 {
            hash += 2;
        }
        let mut pos = probe_position(hash, (*table).slot_count, (*table).slot_count_log2);
        let mut step = STBDS_BUCKET_LENGTH;
        loop {
            let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

            let start = pos & STBDS_BUCKET_MASK;
            for i in start..STBDS_BUCKET_LENGTH {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(
                        a,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        (*bucket).index[i] as usize,
                    ) {
                        return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                    return -1;
                }
            }

            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(
                        a,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        (*bucket).index[i] as usize,
                    ) {
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
}

// --- stbds_hmget_key_ts ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    temp: *mut isize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        let keyoffset: usize = 0;
        if a.is_null() {
            a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            (*header(a)).length += 1;
            memset(a, 0, elemsize);
            *temp = STBDS_INDEX_EMPTY;
            arr_to_hash(a, elemsize)
        } else {
            let raw_a = hash_to_arr(a, elemsize);
            let table = (*header(raw_a)).hash_table as *mut stbds_hash_index;
            if table.is_null() {
                *temp = -1;
            } else {
                let slot = hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
                if slot < 0 {
                    *temp = STBDS_INDEX_EMPTY;
                } else {
                    let b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
                    *temp = (*b).index[(slot as usize) & STBDS_BUCKET_MASK];
                }
            }
            a
        }
    }
}

// --- stbds_hmget_key ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        let mut temp: isize = 0;
        let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp as *mut isize, mode);
        let raw = hash_to_arr(p, elemsize);
        *temp_of(raw) = temp;
        p
    }
}

// --- stbds_hmput_default ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
        if a.is_null() || (*header(hash_to_arr(a, elemsize))).length == 0 {
            let prev = if a.is_null() {
                ptr::null_mut()
            } else {
                hash_to_arr(a, elemsize)
            };
            a = stbds_arrgrowf(prev, elemsize, 0, 1);
            (*header(a)).length += 1;
            memset(a, 0, elemsize);
            a = arr_to_hash(a, elemsize);
        }
        a
    }
}

// --- internal: strdup using realloc ---

unsafe fn stbds_strdup(s: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(s) + 1;
        let p = realloc(ptr::null_mut(), len) as *mut c_char;
        memmove(p as *mut c_void, s as *const c_void, len);
        p
    }
}

// --- stbds_hmput_key ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        let keyoffset: usize = 0;
        let mut raw_a: *mut c_void;

        if a.is_null() {
            a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            memset(a, 0, elemsize);
            (*header(a)).length += 1;
            a = arr_to_hash(a, elemsize);
        }

        raw_a = a;
        a = hash_to_arr(a, elemsize);

        let mut table = (*header(a)).hash_table as *mut stbds_hash_index;

        if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
            let slot_count = if table.is_null() {
                STBDS_BUCKET_LENGTH
            } else {
                (*table).slot_count * 2
            };
            let nt = make_hash_index(slot_count, table);
            if !table.is_null() {
                free(table as *mut c_void);
            } else {
                (*nt).string.mode = if mode >= STBDS_HM_STRING {
                    STBDS_SH_DEFAULT_FOR_STRING as u8
                } else {
                    STBDS_SH_NONE as u8
                };
            }
            (*header(a)).hash_table = nt as *mut c_void;
            table = nt;
        }

        let mut hash = if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut c_char, (*table).seed)
        } else {
            stbds_hash_bytes(key, keysize, (*table).seed)
        };
        let mut step = STBDS_BUCKET_LENGTH;
        let mut tombstone: isize = -1;

        if hash < 2 {
            hash += 2;
        }
        let mut pos = probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

        let pos_final;
        'find: loop {
            let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);

            let start = pos & STBDS_BUCKET_MASK;
            for i in start..STBDS_BUCKET_LENGTH {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(
                        raw_a,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        (*bucket).index[i] as usize,
                    ) {
                        *temp_of(a) = (*bucket).index[i];
                        if mode >= STBDS_HM_STRING {
                            let stored = *((raw_a as *mut u8)
                                .add(elemsize * (*bucket).index[i] as usize + keyoffset)
                                as *mut *mut c_char);
                            *temp_key_of(a) = stored;
                        }
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    pos_final = pos;
                    break 'find;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
                }
            }

            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(
                        raw_a,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        (*bucket).index[i] as usize,
                    ) {
                        *temp_of(a) = (*bucket).index[i];
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    pos_final = pos;
                    break 'find;
                } else if tombstone < 0 {
                    if (*bucket).index[i] == STBDS_INDEX_DELETED {
                        tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                    }
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

        let i: isize = arrlen(a);
        if (i as usize) + 1 > arrcap(a) {
            a = stbds_arrgrowf(a, elemsize, 1, 0);
        }
        raw_a = arr_to_hash(a, elemsize);
        let _ = raw_a; // silence "unused" warning; kept for parity

        (*header(a)).length = (i as usize) + 1;
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
        (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
        *temp_of(a) = i - 1;

        match (*table).string.mode as c_int {
            x if x == STBDS_SH_STRDUP => {
                let dst =
                    (a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char;
                *dst = stbds_strdup(key as *mut c_char);
                *temp_key_of(a) = *dst;
            }
            x if x == STBDS_SH_ARENA => {
                let dst =
                    (a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char;
                *dst = stbds_stralloc(
                    &mut (*table).string as *mut stbds_string_arena,
                    key as *mut c_char,
                );
                *temp_key_of(a) = *dst;
            }
            x if x == STBDS_SH_DEFAULT_FOR_STRING => {
                let dst =
                    (a as *mut u8).add(elemsize * i as usize) as *mut *mut c_char;
                *dst = key as *mut c_char;
                *temp_key_of(a) = *dst;
            }
            _ => {
                memcpy(
                    (a as *mut u8).add(elemsize * i as usize) as *mut c_void,
                    key,
                    keysize,
                );
            }
        }

        arr_to_hash(a, elemsize)
    }
}

// --- stbds_shmode_func ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    unsafe {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*header(a)).length = 1;
        let h = make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
        (*header(a)).hash_table = h as *mut c_void;
        (*h).string.mode = mode as u8;
        arr_to_hash(a, elemsize)
    }
}

// --- stbds_hmdel_key ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        if a.is_null() {
            return ptr::null_mut();
        }
        let raw_a = hash_to_arr(a, elemsize);
        let table = (*header(raw_a)).hash_table as *mut stbds_hash_index;
        *temp_of(raw_a) = 0;
        if table.is_null() {
            return a;
        }
        let mut slot = hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
        if slot < 0 {
            return a;
        }
        let mut b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
        let mut i = (slot as usize) & STBDS_BUCKET_MASK;
        let old_index = (*b).index[i];
        let final_index = arrlen(raw_a) - 1 - 1;

        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        *temp_of(raw_a) = 1;
        (*b).hash[i] = STBDS_HASH_DELETED;
        (*b).index[i] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode as c_int == STBDS_SH_STRDUP {
            let pp =
                (a as *mut u8).add(elemsize * old_index as usize) as *mut *mut c_char;
            free(*pp as *mut c_void);
        }

        if old_index != final_index {
            memmove(
                (a as *mut u8).add(elemsize * old_index as usize) as *mut c_void,
                (a as *mut u8).add(elemsize * final_index as usize) as *const c_void,
                elemsize,
            );

            slot = if mode == STBDS_HM_STRING {
                let stored = *((a as *mut u8)
                    .add(elemsize * old_index as usize + keyoffset)
                    as *mut *mut c_char);
                hm_find_slot(a, elemsize, stored as *mut c_void, keysize, keyoffset, mode)
            } else {
                let kp = (a as *mut u8).add(elemsize * old_index as usize + keyoffset)
                    as *mut c_void;
                hm_find_slot(a, elemsize, kp, keysize, keyoffset, mode)
            };
            b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
            i = (slot as usize) & STBDS_BUCKET_MASK;
            (*b).index[i] = old_index;
        }
        (*header(raw_a)).length -= 1;

        if (*table).used_count < (*table).used_count_shrink_threshold
            && (*table).slot_count > STBDS_BUCKET_LENGTH
        {
            (*header(raw_a)).hash_table =
                make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
            free(table as *mut c_void);
        } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
            (*header(raw_a)).hash_table =
                make_hash_index((*table).slot_count, table) as *mut c_void;
            free(table as *mut c_void);
        }

        a
    }
}

// --- stbds_stralloc ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    unsafe {
        let len = strlen(str_) + 1;
        if len > (*a).remaining {
            let mut blocksize = (*a).block as usize;
            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block += 1;
            }

            if len > blocksize {
                let alloc_size = size_of::<stbds_string_block>() - 8 + len;
                let sb = realloc(ptr::null_mut(), alloc_size) as *mut stbds_string_block;
                memmove(
                    (*sb).storage.as_mut_ptr() as *mut c_void,
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
                return (*sb).storage.as_mut_ptr();
            } else {
                let alloc_size = size_of::<stbds_string_block>() - 8 + blocksize;
                let sb = realloc(ptr::null_mut(), alloc_size) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        let storage = (*(*a).storage).storage.as_mut_ptr();
        let p = storage.add((*a).remaining - len);
        (*a).remaining -= len;
        memmove(p as *mut c_void, str_ as *const c_void, len);
        p
    }
}

// --- stbds_strreset ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    unsafe {
        let mut x = (*a).storage;
        while !x.is_null() {
            let y = (*x).next;
            free(x as *mut c_void);
            x = y;
        }
        memset(a as *mut c_void, 0, size_of::<stbds_string_arena>());
    }
}

// --- strkey ---

// `static char buffer[256]` in C. Single-threaded helper; we replicate the
// behaviour of formatting `"test_%d"` into a 256-byte static buffer and
// returning a pointer to it.
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let fmt = b"test_%d\0".as_ptr() as *const c_char;
        let buf = &raw mut STRKEY_BUFFER as *mut c_char;
        sprintf(buf, fmt, n);
        buf
    }
}

// --- arr_ins (the public entry point) ---
//
// Faithful translation of:
//   for (i=0; i < 5; ++i) {
//     arrpush(arr,1); arrpush(arr,2); arrpush(arr,3); arrpush(arr,4);
//     stbds_arrins(arr,i,num);
//     STBDS_ASSERT(arr[i] == num);
//     if (i < 4)
//       STBDS_ASSERT(arr[4] == 4);
//     arrfree(arr);
//   }

#[inline]
unsafe fn arrmaybegrow(a: *mut c_int, n: usize) -> *mut c_int {
    unsafe {
        if a.is_null()
            || (*header(a as *mut c_void)).length + n > (*header(a as *mut c_void)).capacity
        {
            stbds_arrgrowf(a as *mut c_void, size_of::<c_int>(), n, 0) as *mut c_int
        } else {
            a
        }
    }
}

#[inline]
unsafe fn arrpush(a: *mut c_int, v: c_int) -> *mut c_int {
    unsafe {
        let a = arrmaybegrow(a, 1);
        let len = (*header(a as *mut c_void)).length;
        *a.add(len) = v;
        (*header(a as *mut c_void)).length = len + 1;
        a
    }
}

#[inline]
unsafe fn arrins(a: *mut c_int, i: usize, v: c_int) -> *mut c_int {
    unsafe {
        // arrinsn(a, i, 1): grow by 1, then memmove tail right.
        let a = arrmaybegrow(a, 1);
        (*header(a as *mut c_void)).length += 1;
        let len = (*header(a as *mut c_void)).length;
        memmove(
            a.add(i + 1) as *mut c_void,
            a.add(i) as *const c_void,
            size_of::<c_int>() * (len - 1 - i),
        );
        *a.add(i) = v;
        a
    }
}

#[inline]
unsafe fn arrfree(a: *mut c_int) {
    unsafe {
        if !a.is_null() {
            free(header(a as *mut c_void) as *mut c_void);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_ins(num: c_int) {
    unsafe {
        for i in 0..5usize {
            let mut arr: *mut c_int = ptr::null_mut();

            arr = arrpush(arr, 1);
            arr = arrpush(arr, 2);
            arr = arrpush(arr, 3);
            arr = arrpush(arr, 4);

            arr = arrins(arr, i, num);

            assert!(*arr.add(i) == num);
            if i < 4 {
                assert!(*arr.add(4) == 4);
            }

            arrfree(arr);
        }
    }
}
