#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

type size_t = usize;
type ptrdiff_t = isize;

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;
const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;
const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;
const STBDS_HM_STRING: c_int = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

static STBDS_HASH_SEED: AtomicUsize = AtomicUsize::new(0x31415926);
static mut BUFFER: [c_char; 256] = [0; 256];

#[repr(C)]
struct stbds_array_header {
    length: size_t,
    capacity: size_t,
    hash_table: *mut c_void,
    temp: ptrdiff_t,
}

#[repr(C)]
struct stbds_string_block {
    next: *mut stbds_string_block,
    storage: [c_char; 8],
}

#[repr(C)]
pub struct stbds_string_arena {
    storage: *mut stbds_string_block,
    remaining: size_t,
    block: u8,
    mode: u8,
}

#[repr(C)]
struct stbds_hash_bucket {
    hash: [size_t; STBDS_BUCKET_LENGTH],
    index: [ptrdiff_t; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
struct stbds_hash_index {
    temp_key: *mut c_char,
    slot_count: size_t,
    used_count: size_t,
    used_count_threshold: size_t,
    used_count_shrink_threshold: size_t,
    tombstone_count: size_t,
    tombstone_count_threshold: size_t,
    seed: size_t,
    slot_count_log2: size_t,
    string: stbds_string_arena,
    storage: *mut stbds_hash_bucket,
}

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> size_t;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn memcmp(a: *const c_void, b: *const c_void, n: size_t) -> c_int;
    fn memmove(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    fn memset(dst: *mut c_void, val: c_int, n: size_t) -> *mut c_void;
}

#[inline]
unsafe fn header<T>(t: *mut T) -> *mut stbds_array_header {
    unsafe { (t as *mut stbds_array_header).sub(1) }
}

#[inline]
unsafe fn arrlen(a: *mut c_void) -> ptrdiff_t {
    if a.is_null() {
        0
    } else {
        unsafe { (*header(a)).length as ptrdiff_t }
    }
}

#[inline]
unsafe fn arrcap(a: *mut c_void) -> size_t {
    if a.is_null() {
        0
    } else {
        unsafe { (*header(a)).capacity }
    }
}

#[inline]
unsafe fn hash_to_arr(a: *mut c_void, elemsize: size_t) -> *mut c_void {
    unsafe { (a as *mut u8).sub(elemsize) as *mut c_void }
}

#[inline]
unsafe fn arr_to_hash(a: *mut c_void, elemsize: size_t) -> *mut c_void {
    unsafe { (a as *mut u8).add(elemsize) as *mut c_void }
}

#[inline]
unsafe fn hash_table(a: *mut c_void) -> *mut stbds_hash_index {
    unsafe { (*header(a)).hash_table as *mut stbds_hash_index }
}

#[inline]
unsafe fn temp_key(a: *mut c_void) -> *mut *mut c_char {
    unsafe { &mut (*((*header(a)).hash_table as *mut stbds_hash_index)).temp_key }
}

#[inline]
fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: size_t,
    addlen: size_t,
    mut min_cap: size_t,
) -> *mut c_void {
    unsafe {
        let min_len = (arrlen(a) as size_t).wrapping_add(addlen);
        if min_len > min_cap {
            min_cap = min_len;
        }
        if min_cap <= arrcap(a) {
            return a;
        }
        let old_cap = arrcap(a);
        if min_cap < 2usize.wrapping_mul(old_cap) {
            min_cap = 2usize.wrapping_mul(old_cap);
        } else if min_cap < 4 {
            min_cap = 4;
        }

        let old = if a.is_null() {
            ptr::null_mut()
        } else {
            header(a) as *mut c_void
        };
        let bytes = elemsize
            .wrapping_mul(min_cap)
            .wrapping_add(std::mem::size_of::<stbds_array_header>());
        let base = realloc(old, bytes);
        let b = (base as *mut u8).add(std::mem::size_of::<stbds_array_header>()) as *mut c_void;
        if a.is_null() {
            (*header(b)).length = 0;
            (*header(b)).hash_table = ptr::null_mut();
            (*header(b)).temp = 0;
        }
        (*header(b)).capacity = min_cap;
        b
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    unsafe {
        free(header(a) as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_rand_seed(seed: size_t) {
    STBDS_HASH_SEED.store(seed, Ordering::Relaxed);
}

#[inline]
fn probe_position(hash: size_t, slot_count: size_t, _slot_log2: size_t) -> size_t {
    hash & (slot_count - 1)
}

fn log2(mut slot_count: size_t) -> size_t {
    let mut n = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

unsafe fn make_hash_index(slot_count: size_t, ot: *mut stbds_hash_index) -> *mut stbds_hash_index {
    unsafe {
        let bytes = (slot_count >> STBDS_BUCKET_SHIFT)
            .wrapping_mul(std::mem::size_of::<stbds_hash_bucket>())
            .wrapping_add(std::mem::size_of::<stbds_hash_index>())
            .wrapping_add(STBDS_CACHE_LINE_SIZE - 1);
        let t = realloc(ptr::null_mut(), bytes) as *mut stbds_hash_index;
        (*t).storage = align_fwd(
            t.add(1) as usize,
            STBDS_CACHE_LINE_SIZE,
        ) as *mut stbds_hash_bucket;
        (*t).slot_count = slot_count;
        (*t).slot_count_log2 = log2(slot_count);
        (*t).tombstone_count = 0;
        (*t).used_count = 0;
        (*t).used_count_threshold = slot_count - (slot_count >> 2);
        (*t).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);
        (*t).used_count_shrink_threshold = slot_count >> 2;
        if slot_count <= STBDS_BUCKET_LENGTH {
            (*t).used_count_shrink_threshold = 0;
        }

        if !ot.is_null() {
            ptr::copy_nonoverlapping(&(*ot).string, &mut (*t).string, 1);
            (*t).seed = (*ot).seed;
            (*t).temp_key = ptr::null_mut();
        } else {
            memset(
                &mut (*t).string as *mut _ as *mut c_void,
                0,
                std::mem::size_of::<stbds_string_arena>(),
            );
            (*t).temp_key = ptr::null_mut();
            let seed = STBDS_HASH_SEED.load(Ordering::Relaxed);
            (*t).seed = seed;
            let a = load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
            let b = load_32_or_64(715136305, 0, 0xb504f32d);
            STBDS_HASH_SEED.store(seed.wrapping_mul(a).wrapping_add(b), Ordering::Relaxed);
        }

        for i in 0..(slot_count >> STBDS_BUCKET_SHIFT) {
            let b = (*t).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b).hash[j] = STBDS_HASH_EMPTY;
                (*b).index[j] = STBDS_INDEX_EMPTY;
            }
        }

        if !ot.is_null() {
            (*t).used_count = (*ot).used_count;
            for i in 0..((*ot).slot_count >> STBDS_BUCKET_SHIFT) {
                let ob = (*ot).storage.add(i);
                for j in 0..STBDS_BUCKET_LENGTH {
                    if (*ob).index[j] >= 0 {
                        let hash = (*ob).hash[j];
                        let mut pos = probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                        let mut step = STBDS_BUCKET_LENGTH;
                        'probe: loop {
                            let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);
                            for z in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = (*ob).index[j];
                                    break 'probe;
                                }
                            }
                            let limit = pos & STBDS_BUCKET_MASK;
                            for z in 0..limit {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = (*ob).index[j];
                                    break 'probe;
                                }
                            }
                            pos = pos.wrapping_add(step);
                            step = step.wrapping_add(STBDS_BUCKET_LENGTH);
                            pos &= (*t).slot_count - 1;
                        }
                    }
                }
            }
        }
        t
    }
}

#[inline]
fn load_32_or_64(v32: usize, v64_hi: usize, v64_lo: usize) -> usize {
    let mut temp = v64_lo ^ v32;
    temp <<= 16;
    temp <<= 16;
    temp >>= 16;
    temp >>= 16;
    let mut var = v64_hi;
    var <<= 16;
    var <<= 16;
    var ^ temp ^ v32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: size_t) -> size_t {
    unsafe {
        let mut hash = seed;
        let mut s = str_;
        while *s != 0 {
            hash = hash.rotate_left(9).wrapping_add(*s as u8 as usize);
            s = s.add(1);
        }
        hash ^= seed;
        hash = (!hash).wrapping_add(hash << 18);
        hash ^= hash ^ hash.rotate_right(31);
        hash = hash.wrapping_mul(21);
        hash ^= hash ^ hash.rotate_right(11);
        hash = hash.wrapping_add(hash << 6);
        hash ^= hash.rotate_right(22);
        hash.wrapping_add(seed)
    }
}

unsafe fn siphash_bytes(p: *mut c_void, len: size_t, seed: size_t) -> size_t {
    unsafe {
        let mut d = p as *const u8;
        let mut v0 = (((0x736f6d65usize << 16) << 16) + 0x70736575) ^ seed;
        let mut v1 = (((0x646f7261usize << 16) << 16) + 0x6e646f6d) ^ !seed;
        let mut v2 = (((0x6c796765usize << 16) << 16) + 0x6e657261) ^ seed;
        let mut v3 = (((0x74656462usize << 16) << 16) + 0x79746573) ^ !seed;
        v0 ^= 0x0706050403020100usize ^ seed;
        v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
        v2 ^= 0x0706050403020100usize ^ seed;
        v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

        macro_rules! sipround {
            () => {{
                v0 = v0.wrapping_add(v1);
                v1 = v1.rotate_left(13);
                v1 ^= v0;
                v0 = v0.rotate_left(32);
                v2 = v2.wrapping_add(v3);
                v3 = v3.rotate_left(16);
                v3 ^= v2;
                v2 = v2.wrapping_add(v1);
                v1 = v1.rotate_left(17);
                v1 ^= v2;
                v2 = v2.rotate_left(32);
                v0 = v0.wrapping_add(v3);
                v3 = v3.rotate_left(21);
                v3 ^= v0;
            }};
        }

        let mut i = 0;
        while i + std::mem::size_of::<usize>() <= len {
            let mut data = (*d.add(0) as usize)
                | ((*d.add(1) as usize) << 8)
                | ((*d.add(2) as usize) << 16)
                | ((*d.add(3) as usize) << 24);
            data |= ((*d.add(4) as usize)
                | ((*d.add(5) as usize) << 8)
                | ((*d.add(6) as usize) << 16)
                | ((*d.add(7) as usize) << 24))
                << 32;
            v3 ^= data;
            for _ in 0..2 {
                sipround!();
            }
            v0 ^= data;
            i += std::mem::size_of::<usize>();
            d = d.add(std::mem::size_of::<usize>());
        }

        let mut data = len << (usize::BITS as usize - 8);
        match len - i {
            7 => {
                data |= (*d.add(6) as usize) << 48;
                data |= (*d.add(5) as usize) << 40;
                data |= (*d.add(4) as usize) << 32;
                data |= (*d.add(3) as usize) << 24;
                data |= (*d.add(2) as usize) << 16;
                data |= (*d.add(1) as usize) << 8;
                data |= *d.add(0) as usize;
            }
            6 => {
                data |= (*d.add(5) as usize) << 40;
                data |= (*d.add(4) as usize) << 32;
                data |= (*d.add(3) as usize) << 24;
                data |= (*d.add(2) as usize) << 16;
                data |= (*d.add(1) as usize) << 8;
                data |= *d.add(0) as usize;
            }
            5 => {
                data |= (*d.add(4) as usize) << 32;
                data |= (*d.add(3) as usize) << 24;
                data |= (*d.add(2) as usize) << 16;
                data |= (*d.add(1) as usize) << 8;
                data |= *d.add(0) as usize;
            }
            4 => {
                data |= (*d.add(3) as usize) << 24;
                data |= (*d.add(2) as usize) << 16;
                data |= (*d.add(1) as usize) << 8;
                data |= *d.add(0) as usize;
            }
            3 => {
                data |= (*d.add(2) as usize) << 16;
                data |= (*d.add(1) as usize) << 8;
                data |= *d.add(0) as usize;
            }
            2 => {
                data |= (*d.add(1) as usize) << 8;
                data |= *d.add(0) as usize;
            }
            1 => data |= *d.add(0) as usize,
            _ => {}
        }
        v3 ^= data;
        for _ in 0..2 {
            sipround!();
        }
        v0 ^= data;
        v2 ^= 0xff;
        for _ in 0..4 {
            sipround!();
        }
        v0 ^ v1 ^ v2 ^ v3
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: size_t, seed: size_t) -> size_t {
    unsafe { siphash_bytes(p, len, seed) }
}

unsafe fn is_key_equal(
    a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    keyoffset: size_t,
    mode: c_int,
    i: size_t,
) -> bool {
    unsafe {
        let item_key = (a as *mut u8).add(elemsize.wrapping_mul(i).wrapping_add(keyoffset));
        if mode >= STBDS_HM_STRING {
            strcmp(key as *const c_char, *(item_key as *mut *mut c_char)) == 0
        } else {
            memcmp(key, item_key as *const c_void, keysize) == 0
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: size_t) {
    unsafe {
        if a.is_null() {
            return;
        }
        let table = hash_table(a);
        if !table.is_null() {
            if (*table).string.mode == STBDS_SH_STRDUP {
                for i in 1..(*header(a)).length {
                    free(*(a as *mut u8).add(elemsize * i) as *mut c_void);
                }
            }
            stbds_strreset(&mut (*table).string);
        }
        free((*header(a)).hash_table);
        free(header(a) as *mut c_void);
    }
}

unsafe fn hm_find_slot(
    a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    keyoffset: size_t,
    mode: c_int,
) -> ptrdiff_t {
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
            for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(
                        a,
                        elemsize,
                        key,
                        keysize,
                        keyoffset,
                        mode,
                        (*bucket).index[i] as size_t,
                    ) {
                        return ((pos & !STBDS_BUCKET_MASK) + i) as ptrdiff_t;
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
                        (*bucket).index[i] as size_t,
                    ) {
                        return ((pos & !STBDS_BUCKET_MASK) + i) as ptrdiff_t;
                    }
                } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                    return -1;
                }
            }
            pos = pos.wrapping_add(step);
            step = step.wrapping_add(STBDS_BUCKET_LENGTH);
            pos &= (*table).slot_count - 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    temp: *mut ptrdiff_t,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        if a.is_null() {
            a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            (*header(a)).length += 1;
            memset(a, 0, elemsize);
            *temp = STBDS_INDEX_EMPTY;
            arr_to_hash(a, elemsize)
        } else {
            let raw_a = hash_to_arr(a, elemsize);
            let table = hash_table(raw_a);
            if table.is_null() {
                *temp = -1;
            } else {
                let slot = hm_find_slot(a, elemsize, key, keysize, 0, mode);
                if slot < 0 {
                    *temp = STBDS_INDEX_EMPTY;
                } else {
                    let b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
                    *temp = (*b).index[slot as usize & STBDS_BUCKET_MASK];
                }
            }
            a
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        let mut temp = 0;
        let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
        (*header(hash_to_arr(p, elemsize))).temp = temp;
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: size_t) -> *mut c_void {
    unsafe {
        if a.is_null() || (*header(hash_to_arr(a, elemsize))).length == 0 {
            a = stbds_arrgrowf(
                if a.is_null() { ptr::null_mut() } else { hash_to_arr(a, elemsize) },
                elemsize,
                0,
                1,
            );
            (*header(a)).length += 1;
            memset(a, 0, elemsize);
            a = arr_to_hash(a, elemsize);
        }
        a
    }
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(str_) + 1;
        let p = realloc(ptr::null_mut(), len) as *mut c_char;
        memmove(p as *mut c_void, str_ as *const c_void, len);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        if a.is_null() {
            a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            memset(a, 0, elemsize);
            (*header(a)).length += 1;
            a = arr_to_hash(a, elemsize);
        }

        let mut raw_a = a;
        a = hash_to_arr(a, elemsize);
        let mut table = hash_table(a);
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
                (*nt).string.mode = if mode >= STBDS_HM_STRING { STBDS_SH_DEFAULT } else { 0 };
            }
            (*header(a)).hash_table = nt as *mut c_void;
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
        let mut pos = probe_position(hash, (*table).slot_count, (*table).slot_count_log2);
        let mut step = STBDS_BUCKET_LENGTH;
        let mut tombstone: ptrdiff_t = -1;
        let bucket;
        'search: loop {
            let b = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                if (*b).hash[i] == hash {
                    if is_key_equal(raw_a, elemsize, key, keysize, 0, mode, (*b).index[i] as size_t) {
                        (*header(a)).temp = (*b).index[i];
                        if mode >= STBDS_HM_STRING {
                            *temp_key(a) = *((raw_a as *mut u8)
                                .add(elemsize * (*b).index[i] as usize) as *mut *mut c_char);
                        }
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*b).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    bucket = b;
                    break 'search;
                } else if tombstone < 0 && (*b).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as ptrdiff_t;
                }
            }
            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
                if (*b).hash[i] == hash {
                    if is_key_equal(raw_a, elemsize, key, keysize, 0, mode, (*b).index[i] as size_t) {
                        (*header(a)).temp = (*b).index[i];
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*b).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    bucket = b;
                    break 'search;
                } else if tombstone < 0 && (*b).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as ptrdiff_t;
                }
            }
            pos = pos.wrapping_add(step);
            step = step.wrapping_add(STBDS_BUCKET_LENGTH);
            pos &= (*table).slot_count - 1;
        }

        if tombstone >= 0 {
            pos = tombstone as usize;
            (*table).tombstone_count -= 1;
        }
        (*table).used_count += 1;
        let i = arrlen(a);
        if (i as usize) + 1 > arrcap(a) {
            a = stbds_arrgrowf(a, elemsize, 1, 0);
        }
        raw_a = arr_to_hash(a, elemsize);
        (*header(a)).length = i as usize + 1;
        let b = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        (*b).hash[pos & STBDS_BUCKET_MASK] = hash;
        (*b).index[pos & STBDS_BUCKET_MASK] = i - 1;
        (*header(a)).temp = i - 1;
        let dst = (a as *mut u8).add(elemsize * i as usize);
        match (*table).string.mode {
            STBDS_SH_STRDUP => {
                let p = stbds_strdup(key as *mut c_char);
                *(dst as *mut *mut c_char) = p;
                *temp_key(a) = p;
            }
            STBDS_SH_ARENA => {
                let p = stbds_stralloc(&mut (*table).string, key as *mut c_char);
                *(dst as *mut *mut c_char) = p;
                *temp_key(a) = p;
            }
            STBDS_SH_DEFAULT => {
                *(dst as *mut *mut c_char) = key as *mut c_char;
                *temp_key(a) = key as *mut c_char;
            }
            _ => {
                memmove(dst as *mut c_void, key as *const c_void, keysize);
            }
        }
        let _ = bucket;
        raw_a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: size_t, mode: c_int) -> *mut c_void {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut c_void,
    elemsize: size_t,
    key: *mut c_void,
    keysize: size_t,
    keyoffset: size_t,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        if a.is_null() {
            return ptr::null_mut();
        }
        let raw_a = hash_to_arr(a, elemsize);
        let table = hash_table(raw_a);
        (*header(raw_a)).temp = 0;
        if table.is_null() {
            return a;
        }
        let mut slot = hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
        if slot < 0 {
            return a;
        }
        let mut b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
        let mut i = slot as usize & STBDS_BUCKET_MASK;
        let old_index = (*b).index[i];
        let final_index = arrlen(raw_a) - 2;
        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        (*header(raw_a)).temp = 1;
        (*b).hash[i] = STBDS_HASH_DELETED;
        (*b).index[i] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
            free(*(a as *mut u8).add(elemsize * old_index as usize) as *mut c_void);
        }
        if old_index != final_index {
            memmove(
                (a as *mut u8).add(elemsize * old_index as usize) as *mut c_void,
                (a as *mut u8).add(elemsize * final_index as usize) as *const c_void,
                elemsize,
            );
            let moved_key = if mode == STBDS_HM_STRING {
                *((a as *mut u8).add(elemsize * old_index as usize + keyoffset) as *mut *mut c_char)
                    as *mut c_void
            } else {
                (a as *mut u8).add(elemsize * old_index as usize + keyoffset) as *mut c_void
            };
            slot = hm_find_slot(a, elemsize, moved_key, keysize, keyoffset, mode);
            b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
            i = slot as usize & STBDS_BUCKET_MASK;
            (*b).index[i] = old_index;
        }
        (*header(raw_a)).length -= 1;

        if (*table).used_count < (*table).used_count_shrink_threshold
            && (*table).slot_count > STBDS_BUCKET_LENGTH
        {
            (*header(raw_a)).hash_table = make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
            free(table as *mut c_void);
        } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
            (*header(raw_a)).hash_table = make_hash_index((*table).slot_count, table) as *mut c_void;
            free(table as *mut c_void);
        }
        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    unsafe {
        let len = strlen(str_) + 1;
        if len > (*a).remaining {
            let blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << ((*a).block >> 1);
            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }
            if len > blocksize {
                let sb = realloc(
                    ptr::null_mut(),
                    std::mem::size_of::<stbds_string_block>() - 8 + len,
                ) as *mut stbds_string_block;
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
                let sb = realloc(
                    ptr::null_mut(),
                    std::mem::size_of::<stbds_string_block>() - 8 + blocksize,
                ) as *mut stbds_string_block;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        let p = ((*(*a).storage).storage.as_mut_ptr() as *mut u8)
            .add((*a).remaining - len) as *mut c_char;
        (*a).remaining -= len;
        memmove(p as *mut c_void, str_ as *const c_void, len);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    unsafe {
        let mut x = (*a).storage;
        while !x.is_null() {
            let y = (*x).next;
            free(x as *mut c_void);
            x = y;
        }
        memset(
            a as *mut c_void,
            0,
            std::mem::size_of::<stbds_string_arena>(),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let s = format!("test_{}\0", n);
        let bytes = s.as_bytes();
        let len = bytes.len().min(256);
        let dst = ptr::addr_of_mut!(BUFFER).cast::<c_char>();
        ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, dst, len);
        if len == 256 {
            *dst.add(255) = 0;
        }
        dst
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hm_geti(num: c_int) {
    let mut intmap: HashMap<c_int, c_int> = HashMap::new();

    let mut i: c_int = 1;
    assert!(!intmap.contains_key(&i));
    let default_value: c_int = -2;
    assert!(!intmap.contains_key(&i));
    assert_eq!(*intmap.get(&i).unwrap_or(&default_value), -2);

    i = 0;
    while i < num {
        intmap.insert(i, i.wrapping_mul(5));
        i = i.wrapping_add(2);
    }

    i = 0;
    while i < num {
        let got = *intmap.get(&i).unwrap_or(&default_value);
        if (i & 1) != 0 {
            assert_eq!(got, -2);
        } else {
            assert_eq!(got, i.wrapping_mul(5));
        }
        let got_ts = *intmap.get(&i).unwrap_or(&default_value);
        if (i & 1) != 0 {
            assert_eq!(got_ts, -2);
        } else {
            assert_eq!(got_ts, i.wrapping_mul(5));
        }
        i = i.wrapping_add(1);
    }

    i = 0;
    while i < num {
        intmap.insert(i, i.wrapping_mul(3));
        i = i.wrapping_add(2);
    }

    i = 0;
    while i < num {
        let got = *intmap.get(&i).unwrap_or(&default_value);
        if (i & 1) != 0 {
            assert_eq!(got, -2);
        } else {
            assert_eq!(got, i.wrapping_mul(3));
        }
        i = i.wrapping_add(1);
    }

    i = 2;
    while i < num {
        intmap.remove(&i);
        i = i.wrapping_add(4);
    }

    i = 0;
    while i < num {
        let got = *intmap.get(&i).unwrap_or(&default_value);
        if (i & 3) != 0 {
            assert_eq!(got, -2);
        } else {
            assert_eq!(got, i.wrapping_mul(3));
        }
        i = i.wrapping_add(1);
    }

    i = 0;
    while i < num {
        intmap.remove(&i);
        i = i.wrapping_add(1);
    }

    i = 0;
    while i < num {
        assert_eq!(*intmap.get(&i).unwrap_or(&default_value), -2);
        i = i.wrapping_add(1);
    }

    intmap.clear();
    i = 0;
    while i < num {
        intmap.insert(i, i.wrapping_mul(3));
        i = i.wrapping_add(2);
    }
    intmap.clear();
}
