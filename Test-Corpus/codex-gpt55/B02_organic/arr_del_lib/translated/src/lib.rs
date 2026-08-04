use std::ffi::{c_char, c_int, c_void};
use std::mem::{size_of, zeroed};
use std::ptr::{copy, null_mut};
use std::sync::atomic::{AtomicUsize, Ordering};

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
const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

static STBDS_HASH_SEED: AtomicUsize = AtomicUsize::new(0x31415926);
static mut BUFFER: [c_char; 256] = [0; 256];

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn sprintf(s: *mut c_char, format: *const c_char, ...) -> c_int;
}

#[repr(C)]
struct StbdsArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
struct StbdsStringBlock {
    next: *mut StbdsStringBlock,
    storage: [c_char; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StbdsStringArena {
    storage: *mut StbdsStringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
struct StbdsHashBucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
struct StbdsHashIndex {
    temp_key: *mut c_char,
    slot_count: usize,
    used_count: usize,
    used_count_threshold: usize,
    used_count_shrink_threshold: usize,
    tombstone_count: usize,
    tombstone_count_threshold: usize,
    seed: usize,
    slot_count_log2: usize,
    string: StbdsStringArena,
    storage: *mut StbdsHashBucket,
}

#[inline]
unsafe fn header(a: *mut c_void) -> *mut StbdsArrayHeader {
    unsafe { (a as *mut StbdsArrayHeader).offset(-1) }
}

#[inline]
unsafe fn arrlen(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*header(a)).length }
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
unsafe fn hash_to_arr(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (a as *mut u8).sub(elemsize) as *mut c_void }
}

#[inline]
unsafe fn arr_to_hash(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (a as *mut u8).add(elemsize) as *mut c_void }
}

#[inline]
unsafe fn hash_table(a: *mut c_void) -> *mut StbdsHashIndex {
    unsafe { (*header(a)).hash_table as *mut StbdsHashIndex }
}

#[inline]
unsafe fn temp_key_slot(a: *mut c_void) -> *mut *mut c_char {
    unsafe { (*header(a)).hash_table as *mut *mut c_char }
}

#[inline]
fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

#[inline]
fn probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

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

unsafe fn make_hash_index(slot_count: usize, ot: *mut StbdsHashIndex) -> *mut StbdsHashIndex {
    let bucket_count = slot_count >> STBDS_BUCKET_SHIFT;
    let bytes = bucket_count * size_of::<StbdsHashBucket>()
        + size_of::<StbdsHashIndex>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let t = unsafe { realloc(null_mut(), bytes) as *mut StbdsHashIndex };
    unsafe {
        (*t).storage =
            align_fwd((t.add(1)) as usize, STBDS_CACHE_LINE_SIZE) as *mut StbdsHashBucket;
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
            (*t).string = zeroed();
            (*t).seed = STBDS_HASH_SEED.load(Ordering::Relaxed);
            let a = load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
            let b = load_32_or_64(715136305, 0, 0xb504f32d);
            STBDS_HASH_SEED.store((*t).seed.wrapping_mul(a).wrapping_add(b), Ordering::Relaxed);
        }

        for i in 0..bucket_count {
            let b = (*t).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                (*b).hash[j] = STBDS_HASH_EMPTY;
            }
            for j in 0..STBDS_BUCKET_LENGTH {
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
                        loop {
                            let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);
                            for z in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = (*ob).index[j];
                                    break;
                                }
                            }
                            let placed = (0..STBDS_BUCKET_LENGTH).any(|z| {
                                (*bucket).hash[z] == hash && (*bucket).index[z] == (*ob).index[j]
                            });
                            if placed {
                                break;
                            }
                            for z in 0..(pos & STBDS_BUCKET_MASK) {
                                if (*bucket).hash[z] == 0 {
                                    (*bucket).hash[z] = hash;
                                    (*bucket).index[z] = (*ob).index[j];
                                    break;
                                }
                            }
                            let placed = (0..STBDS_BUCKET_LENGTH).any(|z| {
                                (*bucket).hash[z] == hash && (*bucket).index[z] == (*ob).index[j]
                            });
                            if placed {
                                break;
                            }
                            pos = (pos + step) & ((*t).slot_count - 1);
                            step += STBDS_BUCKET_LENGTH;
                        }
                    }
                }
            }
        }
    }
    t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    unsafe {
        let min_len = arrlen(a).wrapping_add(addlen);
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
        let old = if a.is_null() {
            null_mut()
        } else {
            header(a) as *mut c_void
        };
        let b0 = realloc(old, elemsize * min_cap + size_of::<StbdsArrayHeader>());
        let b = (b0 as *mut u8).add(size_of::<StbdsArrayHeader>()) as *mut c_void;
        if a.is_null() {
            (*header(b)).length = 0;
            (*header(b)).hash_table = null_mut();
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
pub extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED.store(seed, Ordering::Relaxed);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    unsafe {
        let mut p = str_ as *mut u8;
        let mut hash = seed;
        while *p != 0 {
            hash = hash.rotate_left(9).wrapping_add(*p as usize);
            p = p.add(1);
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

unsafe fn siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe {
        let mut d = p as *mut u8;
        let mut v0 = ((((0x736f6d65usize << 16) << 16) + 0x70736575) ^ seed)
            ^ 0x0706050403020100usize
            ^ seed;
        let mut v1 = ((((0x646f7261usize << 16) << 16) + 0x6e646f6d) ^ !seed)
            ^ 0x0f0e0d0c0b0a0908usize
            ^ !seed;
        let mut v2 = ((((0x6c796765usize << 16) << 16) + 0x6e657261) ^ seed)
            ^ 0x0706050403020100usize
            ^ seed;
        let mut v3 = ((((0x74656462usize << 16) << 16) + 0x79746573) ^ !seed)
            ^ 0x0f0e0d0c0b0a0908usize
            ^ !seed;

        macro_rules! sipround {
            () => {{
                v0 = v0.wrapping_add(v1);
                v1 = v1.rotate_left(13);
                v1 ^= v0;
                v0 = v0.rotate_left(usize::BITS / 2);
                v2 = v2.wrapping_add(v3);
                v3 = v3.rotate_left(16);
                v3 ^= v2;
                v2 = v2.wrapping_add(v1);
                v1 = v1.rotate_left(17);
                v1 ^= v2;
                v2 = v2.rotate_left(usize::BITS / 2);
                v0 = v0.wrapping_add(v3);
                v3 = v3.rotate_left(21);
                v3 ^= v0;
            }};
        }

        let mut i = 0;
        while i + size_of::<usize>() <= len {
            let mut data = (*d.add(0) as usize)
                | ((*d.add(1) as usize) << 8)
                | ((*d.add(2) as usize) << 16)
                | ((*d.add(3) as usize) << 24);
            data |= (((*d.add(4) as usize)
                | ((*d.add(5) as usize) << 8)
                | ((*d.add(6) as usize) << 16)
                | ((*d.add(7) as usize) << 24))
                << 16)
                << 16;
            v3 ^= data;
            for _ in 0..2 {
                sipround!();
            }
            v0 ^= data;
            i += size_of::<usize>();
            d = d.add(size_of::<usize>());
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
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { siphash_bytes(p, len, seed) }
}

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
        let item_key = (a as *mut u8).add(elemsize * i + keyoffset) as *mut c_void;
        if mode >= STBDS_HM_STRING {
            strcmp(key as *const c_char, *(item_key as *mut *mut c_char)) == 0
        } else {
            memcmp(key, item_key, keysize) == 0
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    unsafe {
        if a.is_null() {
            return;
        }
        let table = hash_table(a);
        if !table.is_null() {
            if (*table).string.mode == STBDS_SH_STRDUP {
                for i in 1..(*header(a)).length {
                    free(*(a as *mut u8).add(elemsize * i).cast::<*mut c_void>());
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
            for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
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
            for i in 0..(pos & STBDS_BUCKET_MASK) {
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
            pos = (pos + step) & ((*table).slot_count - 1);
            step += STBDS_BUCKET_LENGTH;
        }
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
    unsafe {
        if a.is_null() {
            a = stbds_arrgrowf(null_mut(), elemsize, 0, 1);
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
                    let b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
                    *temp = (*b).index[(slot as usize) & STBDS_BUCKET_MASK];
                }
            }
            a
        }
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
    unsafe {
        let mut temp = 0isize;
        let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
        (*header(hash_to_arr(p, elemsize))).temp = temp;
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
        if a.is_null() || (*header(hash_to_arr(a, elemsize))).length == 0 {
            let raw = if a.is_null() {
                null_mut()
            } else {
                hash_to_arr(a, elemsize)
            };
            a = stbds_arrgrowf(raw, elemsize, 0, 1);
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
        let p = realloc(null_mut(), len) as *mut c_char;
        memmove(p as *mut c_void, str_ as *const c_void, len);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    mut a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        if a.is_null() {
            a = stbds_arrgrowf(null_mut(), elemsize, 0, 1);
            memset(a, 0, elemsize);
            (*header(a)).length += 1;
            a = arr_to_hash(a, elemsize);
        }

        let raw_a = a;
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
                (*nt).string.mode = if mode >= STBDS_HM_STRING { 1 } else { 0 };
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
        let mut tombstone: isize = -1;

        loop {
            let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(
                        raw_a,
                        elemsize,
                        key,
                        keysize,
                        0,
                        mode,
                        (*bucket).index[i] as usize,
                    ) {
                        (*header(a)).temp = (*bucket).index[i];
                        if mode >= STBDS_HM_STRING {
                            *temp_key_slot(a) = *((raw_a as *mut u8)
                                .add(elemsize * ((*bucket).index[i] as usize))
                                as *mut *mut c_char);
                        }
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break;
                } else if tombstone < 0 && (*bucket).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }
            if (*bucket).hash[pos & STBDS_BUCKET_MASK] == 0 {
                break;
            }
            let limit = pos & STBDS_BUCKET_MASK;
            let mut found = false;
            for i in 0..limit {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(
                        raw_a,
                        elemsize,
                        key,
                        keysize,
                        0,
                        mode,
                        (*bucket).index[i] as usize,
                    ) {
                        (*header(a)).temp = (*bucket).index[i];
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    found = true;
                    break;
                } else if tombstone < 0 && (*bucket).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            }
            if found {
                break;
            }
            pos = (pos + step) & ((*table).slot_count - 1);
            step += STBDS_BUCKET_LENGTH;
        }

        if tombstone >= 0 {
            pos = tombstone as usize;
            (*table).tombstone_count -= 1;
        }
        (*table).used_count += 1;

        let i = arrlen(a) as isize;
        if (i as usize) + 1 > arrcap(a) {
            a = stbds_arrgrowf(a, elemsize, 1, 0);
        }
        (*header(a)).length = (i + 1) as usize;
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
        (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
        (*header(a)).temp = i - 1;
        let item = (a as *mut u8).add(elemsize * (i as usize));
        match (*table).string.mode {
            STBDS_SH_STRDUP => {
                let s = stbds_strdup(key as *mut c_char);
                *temp_key_slot(a) = s;
                *(item as *mut *mut c_char) = s;
            }
            STBDS_SH_ARENA => {
                let s = stbds_stralloc(&mut (*table).string, key as *mut c_char);
                *temp_key_slot(a) = s;
                *(item as *mut *mut c_char) = s;
            }
            1 => {
                *temp_key_slot(a) = key as *mut c_char;
                *(item as *mut *mut c_char) = key as *mut c_char;
            }
            _ => {
                copy(key as *const u8, item, keysize);
            }
        }
        arr_to_hash(a, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    unsafe {
        let a = stbds_arrgrowf(null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*header(a)).length = 1;
        let h = make_hash_index(STBDS_BUCKET_LENGTH, null_mut());
        (*header(a)).hash_table = h as *mut c_void;
        (*h).string.mode = mode as u8;
        arr_to_hash(a, elemsize)
    }
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
    unsafe {
        if a.is_null() {
            return null_mut();
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
        let mut b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
        let mut i = (slot as usize) & STBDS_BUCKET_MASK;
        let old_index = (*b).index[i];
        let final_index = arrlen(raw_a) as isize - 2;
        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        (*header(raw_a)).temp = 1;
        (*b).hash[i] = STBDS_HASH_DELETED;
        (*b).index[i] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
            free(*((a as *mut u8).add(elemsize * (old_index as usize)) as *mut *mut c_void));
        }

        if old_index != final_index {
            memmove(
                (a as *mut u8).add(elemsize * (old_index as usize)) as *mut c_void,
                (a as *mut u8).add(elemsize * (final_index as usize)) as *const c_void,
                elemsize,
            );
            if mode == STBDS_HM_STRING {
                let moved_key = *((a as *mut u8).add(elemsize * (old_index as usize) + keyoffset)
                    as *mut *mut c_void);
                slot = hm_find_slot(a, elemsize, moved_key, keysize, keyoffset, mode);
            } else {
                let moved_key =
                    (a as *mut u8).add(elemsize * (old_index as usize) + keyoffset) as *mut c_void;
                slot = hm_find_slot(a, elemsize, moved_key, keysize, keyoffset, mode);
            }
            b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut StbdsStringArena,
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
                let sb = realloc(null_mut(), size_of::<StbdsStringBlock>() - 8 + len)
                    as *mut StbdsStringBlock;
                memmove(
                    (*sb).storage.as_mut_ptr() as *mut c_void,
                    str_ as *const c_void,
                    len,
                );
                if !(*a).storage.is_null() {
                    (*sb).next = (*(*a).storage).next;
                    (*(*a).storage).next = sb;
                } else {
                    (*sb).next = null_mut();
                    (*a).storage = sb;
                    (*a).remaining = 0;
                }
                return (*sb).storage.as_mut_ptr();
            } else {
                let sb = realloc(null_mut(), size_of::<StbdsStringBlock>() - 8 + blocksize)
                    as *mut StbdsStringBlock;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }
        let p = ((*(*a).storage).storage.as_mut_ptr() as *mut u8).add((*a).remaining - len)
            as *mut c_char;
        (*a).remaining -= len;
        memmove(p as *mut c_void, str_ as *const c_void, len);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut StbdsStringArena) {
    unsafe {
        let mut x = (*a).storage;
        while !x.is_null() {
            let y = (*x).next;
            free(x as *mut c_void);
            x = y;
        }
        memset(a as *mut c_void, 0, size_of::<StbdsStringArena>());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let p = (&raw mut BUFFER).cast::<c_char>();
        sprintf(p, c"test_%d".as_ptr(), n);
        p
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_del(num: c_int) {
    unsafe {
        let mut arr: *mut c_int = null_mut();
        for i in 0..4isize {
            for v in [num, 2, 3, 4] {
                arr = stbds_arrgrowf(arr as *mut c_void, size_of::<c_int>(), 1, 0) as *mut c_int;
                let len = (*header(arr as *mut c_void)).length;
                *arr.add(len) = v;
                (*header(arr as *mut c_void)).length = len + 1;
            }
            let len = (*header(arr as *mut c_void)).length;
            memmove(
                arr.offset(i) as *mut c_void,
                arr.offset(i + 1) as *const c_void,
                size_of::<c_int>() * (len - 1 - i as usize),
            );
            (*header(arr as *mut c_void)).length -= 1;
            free(header(arr as *mut c_void) as *mut c_void);
            arr = null_mut();

            for v in [num, 2, 3, 4] {
                arr = stbds_arrgrowf(arr as *mut c_void, size_of::<c_int>(), 1, 0) as *mut c_int;
                let len = (*header(arr as *mut c_void)).length;
                *arr.add(len) = v;
                (*header(arr as *mut c_void)).length = len + 1;
            }
            let len = (*header(arr as *mut c_void)).length;
            *arr.offset(i) = *arr.add(len - 1);
            (*header(arr as *mut c_void)).length -= 1;
            free(header(arr as *mut c_void) as *mut c_void);
            arr = null_mut();
        }
    }
}
