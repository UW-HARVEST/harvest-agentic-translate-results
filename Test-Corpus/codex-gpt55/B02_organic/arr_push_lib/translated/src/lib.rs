use std::ffi::{c_char, c_int, c_void};
use std::ptr;

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_HM_STRING: c_int = 1;

const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[repr(C)]
struct ArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
struct StringBlock {
    next: *mut StringBlock,
    storage: [c_char; 8],
}

#[repr(C)]
pub struct StringArena {
    storage: *mut StringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
}

#[repr(C)]
struct HashBucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
struct HashIndex {
    temp_key: *mut c_char,
    slot_count: usize,
    used_count: usize,
    used_count_threshold: usize,
    used_count_shrink_threshold: usize,
    tombstone_count: usize,
    tombstone_count_threshold: usize,
    seed: usize,
    slot_count_log2: usize,
    string: StringArena,
    storage: *mut HashBucket,
}

#[link(name = "c")]
unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
}

static mut STBDS_HASH_SEED: usize = 0x31415926;
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

#[inline]
unsafe fn header<T>(a: *mut T) -> *mut ArrayHeader {
    (a as *mut ArrayHeader).offset(-1)
}

#[inline]
unsafe fn arr_len(a: *mut c_void) -> isize {
    if a.is_null() {
        0
    } else {
        (*header(a)).length as isize
    }
}

#[inline]
unsafe fn arr_cap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*header(a)).capacity
    }
}

#[inline]
unsafe fn hash_to_arr(a: *mut c_void, elemsize: usize) -> *mut c_void {
    (a as *mut u8).sub(elemsize) as *mut c_void
}

#[inline]
unsafe fn arr_to_hash(a: *mut c_void, elemsize: usize) -> *mut c_void {
    (a as *mut u8).add(elemsize) as *mut c_void
}

#[inline]
unsafe fn hash_table(a: *mut c_void) -> *mut HashIndex {
    (*header(a)).hash_table as *mut HashIndex
}

#[inline]
fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

#[inline]
fn rotate_left(v: usize, n: u32) -> usize {
    v.rotate_left(n)
}

#[inline]
fn rotate_right(v: usize, n: u32) -> usize {
    v.rotate_right(n)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = (arr_len(a) as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }
    if min_cap <= arr_cap(a) {
        return a;
    }

    let cap = arr_cap(a);
    if min_cap < 2usize.wrapping_mul(cap) {
        min_cap = 2usize.wrapping_mul(cap);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let base = if a.is_null() {
        ptr::null_mut()
    } else {
        header(a) as *mut c_void
    };
    let bytes = elemsize
        .wrapping_mul(min_cap)
        .wrapping_add(std::mem::size_of::<ArrayHeader>());
    let raw = realloc(base, bytes);
    let b = (raw as *mut u8).add(std::mem::size_of::<ArrayHeader>()) as *mut c_void;

    if a.is_null() {
        (*header(b)).length = 0;
        (*header(b)).hash_table = ptr::null_mut();
        (*header(b)).temp = 0;
    }
    (*header(b)).capacity = min_cap;
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(header(a) as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED = seed;
}

fn probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn log2(mut slot_count: usize) -> usize {
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

unsafe fn make_hash_index(slot_count: usize, ot: *mut HashIndex) -> *mut HashIndex {
    let bucket_count = slot_count >> STBDS_BUCKET_SHIFT;
    let bytes = bucket_count
        .wrapping_mul(std::mem::size_of::<HashBucket>())
        .wrapping_add(std::mem::size_of::<HashIndex>())
        .wrapping_add(STBDS_CACHE_LINE_SIZE - 1);
    let t = realloc(ptr::null_mut(), bytes) as *mut HashIndex;
    (*t).storage = align_fwd(
        (t.add(1)) as usize,
        STBDS_CACHE_LINE_SIZE,
    ) as *mut HashBucket;
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
    } else {
        ptr::write_bytes(&mut (*t).string as *mut StringArena as *mut u8, 0, std::mem::size_of::<StringArena>());
        (*t).seed = STBDS_HASH_SEED;
        let a = load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
        let b = load_32_or_64(715136305, 0, 0xb504f32d);
        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
    }

    for i in 0..bucket_count {
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
                    'search: loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);
                        for z in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'search;
                            }
                        }
                        let limit = pos & STBDS_BUCKET_MASK;
                        for z in 0..limit {
                            if (*bucket).hash[z] == 0 {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = (*ob).index[j];
                                break 'search;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    let mut p = str_;
    while *p != 0 {
        hash = rotate_left(hash, 9).wrapping_add(*p as u8 as usize);
        p = p.add(1);
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash << 18);
    hash ^= hash ^ rotate_right(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ rotate_right(hash, 11);
    hash = hash.wrapping_add(hash << 6);
    hash ^= rotate_right(hash, 22);
    hash.wrapping_add(seed)
}

unsafe fn siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *mut u8;
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
            v1 = rotate_left(v1, 13);
            v1 ^= v0;
            v0 = rotate_left(v0, (usize::BITS / 2) as u32);
            v2 = v2.wrapping_add(v3);
            v3 = rotate_left(v3, 16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = rotate_left(v1, 17);
            v1 ^= v2;
            v2 = rotate_left(v2, (usize::BITS / 2) as u32);
            v0 = v0.wrapping_add(v3);
            v3 = rotate_left(v3, 21);
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
            << 16
            << 16;

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
            data |= (*d.add(6) as usize) << 24 << 24;
            data |= (*d.add(5) as usize) << 20 << 20;
            data |= (*d.add(4) as usize) << 16 << 16;
            data |= (*d.add(3) as usize) << 24;
            data |= (*d.add(2) as usize) << 16;
            data |= (*d.add(1) as usize) << 8;
            data |= *d.add(0) as usize;
        }
        6 => {
            data |= (*d.add(5) as usize) << 20 << 20;
            data |= (*d.add(4) as usize) << 16 << 16;
            data |= (*d.add(3) as usize) << 24;
            data |= (*d.add(2) as usize) << 16;
            data |= (*d.add(1) as usize) << 8;
            data |= *d.add(0) as usize;
        }
        5 => {
            data |= (*d.add(4) as usize) << 16 << 16;
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
        1 => {
            data |= *d.add(0) as usize;
        }
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    siphash_bytes(p, len, seed)
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
    if mode >= STBDS_HM_STRING {
        let stored = *((a as *mut u8).add(elemsize * i + keyoffset) as *mut *mut c_char);
        strcmp(key as *const c_char, stored as *const c_char) == 0
    } else {
        let stored = (a as *mut u8).add(elemsize * i + keyoffset);
        std::slice::from_raw_parts(key as *const u8, keysize)
            == std::slice::from_raw_parts(stored as *const u8, keysize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: usize) {
    if a.is_null() {
        return;
    }
    let table = hash_table(a);
    if !table.is_null() {
        if (*table).string.mode == STBDS_SH_STRDUP {
            for i in 1..(*header(a)).length {
                let p = *((a as *mut u8).add(elemsize * i) as *mut *mut c_void);
                free(p);
            }
        }
        stbds_strreset(&mut (*table).string);
    }
    free((*header(a)).hash_table);
    free(header(a) as *mut c_void);
}

unsafe fn hm_find_slot(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> isize {
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
                if is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as usize) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }

        let limit = pos & STBDS_BUCKET_MASK;
        for i in 0..limit {
            if (*bucket).hash[i] == hash {
                if is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as usize) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
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
        (*header(a)).length += 1;
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        *temp = STBDS_INDEX_EMPTY;
        arr_to_hash(a, elemsize)
    } else {
        let raw_a = hash_to_arr(a, elemsize);
        let table = (*header(raw_a)).hash_table as *mut HashIndex;
        if table.is_null() {
            *temp = -1;
        } else {
            let slot = hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    let mut temp = 0isize;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    (*header(hash_to_arr(p, elemsize))).temp = temp;
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null() || (*header(hash_to_arr(a, elemsize))).length == 0 {
        a = stbds_arrgrowf(
            if a.is_null() { ptr::null_mut() } else { hash_to_arr(a, elemsize) },
            elemsize,
            0,
            1,
        );
        (*header(a)).length += 1;
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        a = arr_to_hash(a, elemsize);
    }
    a
}

unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = strlen(str_) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    ptr::copy(str_, p, len);
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
    let keyoffset = 0;

    if a.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        ptr::write_bytes(a as *mut u8, 0, elemsize);
        (*header(a)).length += 1;
        a = arr_to_hash(a, elemsize);
    }

    let mut raw_a = a;
    a = hash_to_arr(a, elemsize);
    let mut table = (*header(a)).hash_table as *mut HashIndex;

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
    let mut tombstone: isize = -1;

    loop {
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
            if (*bucket).hash[i] == hash {
                if is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as usize) {
                    (*header(a)).temp = (*bucket).index[i];
                    if mode >= STBDS_HM_STRING {
                        (*header(a)).hash_table = table as *mut c_void;
                        (*table).temp_key = *((raw_a as *mut u8)
                            .add(elemsize * ((*bucket).index[i] as usize) + keyoffset)
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
        if pos & STBDS_BUCKET_MASK < STBDS_BUCKET_LENGTH {
            let bucket2 = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            if (*bucket2).hash[pos & STBDS_BUCKET_MASK] == 0 {
                break;
            }
        }

        let limit = pos & STBDS_BUCKET_MASK;
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        for i in 0..limit {
            if (*bucket).hash[i] == hash {
                if is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as usize) {
                    (*header(a)).temp = (*bucket).index[i];
                    return arr_to_hash(a, elemsize);
                }
            } else if (*bucket).hash[i] == 0 {
                pos = (pos & !STBDS_BUCKET_MASK) + i;
                break;
            } else if tombstone < 0 && (*bucket).index[i] == STBDS_INDEX_DELETED {
                tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
            }
        }
        let bucket3 = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        if (*bucket3).hash[pos & STBDS_BUCKET_MASK] == 0 {
            break;
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

    let i = arr_len(a);
    if (i as usize) + 1 > arr_cap(a) {
        a = stbds_arrgrowf(a, elemsize, 1, 0);
    }
    raw_a = arr_to_hash(a, elemsize);
    (*header(a)).length = (i as usize) + 1;
    let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
    (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
    (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
    (*header(a)).temp = i - 1;

    let dest = (a as *mut u8).add(elemsize * (i as usize));
    match (*table).string.mode {
        STBDS_SH_STRDUP => {
            let p = stbds_strdup(key as *mut c_char);
            *(dest as *mut *mut c_char) = p;
            (*table).temp_key = p;
        }
        STBDS_SH_ARENA => {
            let p = stbds_stralloc(&mut (*table).string, key as *mut c_char);
            *(dest as *mut *mut c_char) = p;
            (*table).temp_key = p;
        }
        STBDS_SH_DEFAULT => {
            *(dest as *mut *mut c_char) = key as *mut c_char;
            (*table).temp_key = key as *mut c_char;
        }
        _ => ptr::copy_nonoverlapping(key as *const u8, dest, keysize),
    }

    raw_a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    ptr::write_bytes(a as *mut u8, 0, elemsize);
    (*header(a)).length = 1;
    let h = make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*header(a)).hash_table = h as *mut c_void;
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
        return ptr::null_mut();
    }

    let raw_a = hash_to_arr(a, elemsize);
    let table = (*header(raw_a)).hash_table as *mut HashIndex;
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
    let final_index = arr_len(raw_a) - 1 - 1;

    (*table).used_count -= 1;
    (*table).tombstone_count += 1;
    (*header(raw_a)).temp = 1;
    (*b).hash[i] = STBDS_HASH_DELETED;
    (*b).index[i] = STBDS_INDEX_DELETED;

    if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
        let p = *((a as *mut u8).add(elemsize * (old_index as usize)) as *mut *mut c_void);
        free(p);
    }

    if old_index != final_index {
        ptr::copy(
            (a as *mut u8).add(elemsize * (final_index as usize)),
            (a as *mut u8).add(elemsize * (old_index as usize)),
            elemsize,
        );

        slot = if mode == STBDS_HM_STRING {
            let moved_key = *((a as *mut u8)
                .add(elemsize * (old_index as usize) + keyoffset)
                as *mut *mut c_void);
            hm_find_slot(a, elemsize, moved_key, keysize, keyoffset, mode)
        } else {
            hm_find_slot(
                a,
                elemsize,
                (a as *mut u8).add(elemsize * (old_index as usize) + keyoffset) as *mut c_void,
                keysize,
                keyoffset,
                mode,
            )
        };
        b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
        i = (slot as usize) & STBDS_BUCKET_MASK;
        (*b).index[i] = old_index;
    }

    (*header(raw_a)).length -= 1;

    if (*table).used_count < (*table).used_count_shrink_threshold && (*table).slot_count > STBDS_BUCKET_LENGTH {
        (*header(raw_a)).hash_table = make_hash_index((*table).slot_count >> 1, table) as *mut c_void;
        free(table as *mut c_void);
    } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
        (*header(raw_a)).hash_table = make_hash_index((*table).slot_count, table) as *mut c_void;
        free(table as *mut c_void);
    }

    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut StringArena, str_: *mut c_char) -> *mut c_char {
    let len = strlen(str_) + 1;
    if len > (*a).remaining {
        let mut blocksize = (*a).block as usize;
        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb = realloc(
                ptr::null_mut(),
                std::mem::size_of::<StringBlock>() - 8 + len,
            ) as *mut StringBlock;
            ptr::copy(str_, (*sb).storage.as_mut_ptr(), len);
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
                std::mem::size_of::<StringBlock>() - 8 + blocksize,
            ) as *mut StringBlock;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    let p = ((*(*a).storage).storage.as_mut_ptr() as *mut u8)
        .add((*a).remaining - len) as *mut c_char;
    (*a).remaining -= len;
    ptr::copy(str_, p, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut StringArena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        free(x as *mut c_void);
        x = y;
    }
    ptr::write_bytes(a as *mut u8, 0, std::mem::size_of::<StringArena>());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let s = format!("test_{}", n);
    let bytes = s.as_bytes();
    let len = bytes.len().min(255);
    let base = ptr::addr_of_mut!(STRKEY_BUFFER) as *mut c_char;
    for i in 0..len {
        *base.add(i) = bytes[i] as c_char;
    }
    *base.add(len) = 0;
    base
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_push(num: c_int) {
    let mut arr: *mut c_int = ptr::null_mut();
    let mut i = 0;
    while i < num {
        let mut j = 0;
        while j < i {
            if arr.is_null() || (*header(arr)).length + 1 > (*header(arr)).capacity {
                arr = stbds_arrgrowf(arr as *mut c_void, std::mem::size_of::<c_int>(), 1, 0) as *mut c_int;
            }
            let idx = (*header(arr)).length;
            *arr.add(idx) = j;
            (*header(arr)).length += 1;
            j += 1;
        }
        if !arr.is_null() {
            free(header(arr) as *mut c_void);
            arr = ptr::null_mut();
        }
        i += 50;
    }
}
