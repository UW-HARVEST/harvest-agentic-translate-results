use std::ffi::{c_char, c_int, c_void};
use std::mem::{size_of, zeroed};
use std::ptr::{copy_nonoverlapping, null_mut};

type SizeT = usize;
type PtrDiffT = isize;

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;
const STBDS_INDEX_EMPTY: PtrDiffT = -1;
const STBDS_INDEX_DELETED: PtrDiffT = -2;
const STBDS_HASH_EMPTY: SizeT = 0;
const STBDS_HASH_DELETED: SizeT = 1;
const STBDS_HM_STRING: c_int = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[repr(C)]
struct StbdsArrayHeader {
    length: SizeT,
    capacity: SizeT,
    hash_table: *mut c_void,
    temp: PtrDiffT,
}

#[repr(C)]
struct StbdsStringBlock {
    next: *mut StbdsStringBlock,
    storage: [c_char; 8],
}

#[repr(C)]
pub struct StbdsStringArena {
    storage: *mut StbdsStringBlock,
    remaining: SizeT,
    block: u8,
    mode: u8,
}

#[repr(C)]
struct StbdsHashBucket {
    hash: [SizeT; STBDS_BUCKET_LENGTH],
    index: [PtrDiffT; STBDS_BUCKET_LENGTH],
}

#[repr(C)]
struct StbdsHashIndex {
    temp_key: *mut c_char,
    slot_count: SizeT,
    used_count: SizeT,
    used_count_threshold: SizeT,
    used_count_shrink_threshold: SizeT,
    tombstone_count: SizeT,
    tombstone_count_threshold: SizeT,
    seed: SizeT,
    slot_count_log2: SizeT,
    string: StbdsStringArena,
    storage: *mut StbdsHashBucket,
}

#[repr(C)]
struct IntEntry {
    key: c_int,
    value: c_int,
}

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn sprintf(s: *mut c_char, format: *const c_char, ...) -> c_int;
    fn __assert_fail(
        assertion: *const c_char,
        file: *const c_char,
        line: c_int,
        function: *const c_char,
    ) -> !;
}

static mut STBDS_HASH_SEED: SizeT = 0x31415926;
static mut BUFFER: [c_char; 256] = [0; 256];

#[inline]
unsafe fn header(a: *mut c_void) -> *mut StbdsArrayHeader {
    unsafe { (a as *mut StbdsArrayHeader).sub(1) }
}

#[inline]
unsafe fn arr_len(a: *mut c_void) -> PtrDiffT {
    if a.is_null() {
        0
    } else {
        unsafe { (*header(a)).length as PtrDiffT }
    }
}

#[inline]
unsafe fn arr_cap(a: *mut c_void) -> SizeT {
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
unsafe fn hash_table(raw_a: *mut c_void) -> *mut StbdsHashIndex {
    unsafe { (*header(raw_a)).hash_table as *mut StbdsHashIndex }
}

#[inline]
unsafe fn temp_key(raw_a: *mut c_void) -> *mut *mut c_char {
    unsafe { &mut (*(hash_table(raw_a))).temp_key }
}

#[inline]
fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

#[inline]
fn rotate_left(val: usize, n: u32) -> usize {
    val.rotate_left(n)
}

#[inline]
fn rotate_right(val: usize, n: u32) -> usize {
    val.rotate_right(n)
}

#[inline]
fn probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

fn log2_size(mut slot_count: usize) -> usize {
    let mut n = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

unsafe fn make_hash_index(slot_count: usize, ot: *mut StbdsHashIndex) -> *mut StbdsHashIndex {
    let bytes = (slot_count >> STBDS_BUCKET_SHIFT) * size_of::<StbdsHashBucket>()
        + size_of::<StbdsHashIndex>()
        + STBDS_CACHE_LINE_SIZE
        - 1;
    let t = unsafe { realloc(null_mut(), bytes) as *mut StbdsHashIndex };
    unsafe {
        (*t).storage =
            align_fwd(t.add(1) as usize, STBDS_CACHE_LINE_SIZE) as *mut StbdsHashBucket;
        (*t).slot_count = slot_count;
        (*t).slot_count_log2 = log2_size(slot_count);
        (*t).tombstone_count = 0;
        (*t).used_count = 0;
        (*t).used_count_threshold = slot_count - (slot_count >> 2);
        (*t).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);
        (*t).used_count_shrink_threshold = slot_count >> 2;
        if slot_count <= STBDS_BUCKET_LENGTH {
            (*t).used_count_shrink_threshold = 0;
        }

        if !ot.is_null() {
            copy_nonoverlapping(&(*ot).string, &mut (*t).string, 1);
            (*t).seed = (*ot).seed;
        } else {
            memset(
                &mut (*t).string as *mut StbdsStringArena as *mut c_void,
                0,
                size_of::<StbdsStringArena>(),
            );
            (*t).seed = STBDS_HASH_SEED;
            let a = load_32_or_64(2147001325, 0x27bb2ee6, 0x87b0b0fd);
            let b = load_32_or_64(715136305, 0, 0xb504f32d);
            STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a).wrapping_add(b);
        }

        for i in 0..(slot_count >> STBDS_BUCKET_SHIFT) {
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
                            if (pos & STBDS_BUCKET_MASK..STBDS_BUCKET_LENGTH)
                                .any(|z| (*bucket).hash[z] == hash && (*bucket).index[z] == (*ob).index[j])
                            {
                                break;
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
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: SizeT,
    addlen: SizeT,
    mut min_cap: SizeT,
) -> *mut c_void {
    let min_len = unsafe { arr_len(a) as usize }.wrapping_add(addlen);
    if min_len > min_cap {
        min_cap = min_len;
    }
    if min_cap <= unsafe { arr_cap(a) } {
        return a;
    }
    let cap = unsafe { arr_cap(a) };
    if min_cap < 2 * cap {
        min_cap = 2 * cap;
    } else if min_cap < 4 {
        min_cap = 4;
    }
    let old = if a.is_null() {
        null_mut()
    } else {
        unsafe { header(a) as *mut c_void }
    };
    let b = unsafe { realloc(old, elemsize * min_cap + size_of::<StbdsArrayHeader>()) };
    let b = unsafe { (b as *mut u8).add(size_of::<StbdsArrayHeader>()) as *mut c_void };
    unsafe {
        if a.is_null() {
            (*header(b)).length = 0;
            (*header(b)).hash_table = null_mut();
            (*header(b)).temp = 0;
        }
        (*header(b)).capacity = min_cap;
    }
    b
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    unsafe { free(header(a) as *mut c_void) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: SizeT) {
    unsafe { STBDS_HASH_SEED = seed };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: SizeT) -> SizeT {
    let mut hash = seed;
    let mut p = str_;
    unsafe {
        while *p != 0 {
            hash = rotate_left(hash, 9).wrapping_add(*p as u8 as usize);
            p = p.add(1);
        }
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

unsafe fn siphash_bytes(p: *mut c_void, len: SizeT, seed: SizeT) -> SizeT {
    let mut d = p as *mut u8;
    let mut v0 = ((((0x736f6d65usize << 16) << 16) + 0x70736575) ^ seed);
    let mut v1 = ((((0x646f7261usize << 16) << 16) + 0x6e646f6d) ^ !seed);
    let mut v2 = ((((0x6c796765usize << 16) << 16) + 0x6e657261) ^ seed);
    let mut v3 = ((((0x74656462usize << 16) << 16) + 0x79746573) ^ !seed);

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
    while i + size_of::<usize>() <= len {
        let data = unsafe {
            ((*d.add(0) as usize)
                | ((*d.add(1) as usize) << 8)
                | ((*d.add(2) as usize) << 16)
                | ((*d.add(3) as usize) << 24))
                | (((*d.add(4) as usize)
                    | ((*d.add(5) as usize) << 8)
                    | ((*d.add(6) as usize) << 16)
                    | ((*d.add(7) as usize) << 24))
                    << 32)
        };
        v3 ^= data;
        for _ in 0..2 {
            sipround!();
        }
        v0 ^= data;
        i += size_of::<usize>();
        unsafe { d = d.add(size_of::<usize>()) };
    }

    let mut data = len << (usize::BITS as usize - 8);
    unsafe {
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
pub unsafe extern "C" fn stbds_hash_bytes(
    p: *mut c_void,
    len: SizeT,
    seed: SizeT,
) -> SizeT {
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
        if mode >= STBDS_HM_STRING {
            strcmp(
                key as *const c_char,
                *((a as *mut u8).add(elemsize * i + keyoffset) as *mut *mut c_char),
            ) == 0
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
) -> PtrDiffT {
    let raw_a = unsafe { hash_to_arr(a, elemsize) };
    let table = unsafe { hash_table(raw_a) };
    let mut hash = unsafe {
        if mode >= STBDS_HM_STRING {
            stbds_hash_string(key as *mut c_char, (*table).seed)
        } else {
            stbds_hash_bytes(key, keysize, (*table).seed)
        }
    };
    if hash < 2 {
        hash += 2;
    }
    let mut pos = unsafe { probe_position(hash, (*table).slot_count, (*table).slot_count_log2) };
    let mut step = STBDS_BUCKET_LENGTH;
    loop {
        let bucket = unsafe { (*table).storage.add(pos >> STBDS_BUCKET_SHIFT) };
        unsafe {
            for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as usize) {
                        return ((pos & !STBDS_BUCKET_MASK) + i) as PtrDiffT;
                    }
                } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                    return -1;
                }
            }
            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as usize) {
                        return ((pos & !STBDS_BUCKET_MASK) + i) as PtrDiffT;
                    }
                } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                    return -1;
                }
            }
            pos = (pos + step) & ((*table).slot_count - 1);
        }
        step += STBDS_BUCKET_LENGTH;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, elemsize: SizeT) {
    if a.is_null() {
        return;
    }
    unsafe {
        let table = hash_table(a);
        if !table.is_null() {
            if (*table).string.mode == STBDS_SH_STRDUP {
                for i in 1..(*header(a)).length {
                    free(*((a as *mut u8).add(elemsize * i) as *mut *mut c_char) as *mut c_void);
                }
            }
            stbds_strreset(&mut (*table).string);
        }
        free((*header(a)).hash_table);
        free(header(a) as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    mut a: *mut c_void,
    elemsize: SizeT,
    key: *mut c_void,
    keysize: SizeT,
    temp: *mut PtrDiffT,
    mode: c_int,
) -> *mut c_void {
    let keyoffset = 0usize;
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
                let slot = hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
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
    elemsize: SizeT,
    key: *mut c_void,
    keysize: SizeT,
    mode: c_int,
) -> *mut c_void {
    let mut temp: PtrDiffT = 0;
    let p = unsafe { stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode) };
    unsafe {
        (*header(hash_to_arr(p, elemsize))).temp = temp;
    }
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(mut a: *mut c_void, elemsize: SizeT) -> *mut c_void {
    unsafe {
        if a.is_null() || (*header(hash_to_arr(a, elemsize))).length == 0 {
            a = stbds_arrgrowf(
                if a.is_null() { null_mut() } else { hash_to_arr(a, elemsize) },
                elemsize,
                0,
                1,
            );
            (*header(a)).length += 1;
            memset(a, 0, elemsize);
            a = arr_to_hash(a, elemsize);
        }
    }
    a
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
    elemsize: SizeT,
    key: *mut c_void,
    keysize: SizeT,
    mode: c_int,
) -> *mut c_void {
    let keyoffset = 0usize;
    unsafe {
        if a.is_null() {
            a = stbds_arrgrowf(null_mut(), elemsize, 0, 1);
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
        let mut tombstone: PtrDiffT = -1;

        loop {
            let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as usize) {
                        (*header(a)).temp = (*bucket).index[i];
                        if mode >= STBDS_HM_STRING {
                            *temp_key(a) = *((raw_a as *mut u8)
                                .add(elemsize * (*bucket).index[i] as usize + keyoffset)
                                as *mut *mut c_char);
                        }
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break;
                } else if tombstone < 0 && (*bucket).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as PtrDiffT;
                }
            }
            if (*bucket).hash[pos & STBDS_BUCKET_MASK] == 0 {
                break;
            }
            let limit = pos & STBDS_BUCKET_MASK;
            let mut found_empty = false;
            for i in 0..limit {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as usize) {
                        (*header(a)).temp = (*bucket).index[i];
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    found_empty = true;
                    break;
                } else if tombstone < 0 && (*bucket).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as PtrDiffT;
                }
            }
            if found_empty {
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

        let i = arr_len(a);
        if (i as usize) + 1 > arr_cap(a) {
            a = stbds_arrgrowf(a, elemsize, 1, 0);
        }
        raw_a = arr_to_hash(a, elemsize);
        (*header(a)).length = i as usize + 1;
        let bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
        (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
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
        raw_a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: SizeT, mode: c_int) -> *mut c_void {
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
    elemsize: SizeT,
    key: *mut c_void,
    keysize: SizeT,
    keyoffset: SizeT,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        if a.is_null() {
            return null_mut();
        }
        let raw_a = hash_to_arr(a, elemsize);
        let mut table = hash_table(raw_a);
        (*header(raw_a)).temp = 0;
        if table.is_null() {
            return a;
        }
        let mut slot = hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
        if slot < 0 {
            return a;
        }
        let mut b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
        let mut bi = slot as usize & STBDS_BUCKET_MASK;
        let old_index = (*b).index[bi];
        let final_index = arr_len(raw_a) - 1 - 1;
        (*table).used_count -= 1;
        (*table).tombstone_count += 1;
        (*header(raw_a)).temp = 1;
        (*b).hash[bi] = STBDS_HASH_DELETED;
        (*b).index[bi] = STBDS_INDEX_DELETED;

        if mode == STBDS_HM_STRING && (*table).string.mode == STBDS_SH_STRDUP {
            free(*((a as *mut u8).add(elemsize * old_index as usize) as *mut *mut c_char) as *mut c_void);
        }

        if old_index != final_index {
            memmove(
                (a as *mut u8).add(elemsize * old_index as usize) as *mut c_void,
                (a as *mut u8).add(elemsize * final_index as usize) as *const c_void,
                elemsize,
            );
            slot = if mode == STBDS_HM_STRING {
                hm_find_slot(
                    a,
                    elemsize,
                    *((a as *mut u8).add(elemsize * old_index as usize + keyoffset) as *mut *mut c_char)
                        as *mut c_void,
                    keysize,
                    keyoffset,
                    mode,
                )
            } else {
                hm_find_slot(
                    a,
                    elemsize,
                    (a as *mut u8).add(elemsize * old_index as usize + keyoffset) as *mut c_void,
                    keysize,
                    keyoffset,
                    mode,
                )
            };
            b = (*table).storage.add(slot as usize >> STBDS_BUCKET_SHIFT);
            bi = slot as usize & STBDS_BUCKET_MASK;
            (*b).index[bi] = old_index;
        }
        (*header(raw_a)).length -= 1;

        if (*table).used_count < (*table).used_count_shrink_threshold
            && (*table).slot_count > STBDS_BUCKET_LENGTH
        {
            let nt = make_hash_index((*table).slot_count >> 1, table);
            (*header(raw_a)).hash_table = nt as *mut c_void;
            free(table as *mut c_void);
        } else if (*table).tombstone_count > (*table).tombstone_count_threshold {
            let nt = make_hash_index((*table).slot_count, table);
            (*header(raw_a)).hash_table = nt as *mut c_void;
            free(table as *mut c_void);
            table = nt;
            let _ = table;
        }
        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut StbdsStringArena, str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(str_) + 1;
        if len > (*a).remaining {
            let mut blocksize = (*a).block as usize;
            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);
            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }
            if len > blocksize {
                let sb =
                    realloc(null_mut(), size_of::<StbdsStringBlock>() - 8 + len) as *mut StbdsStringBlock;
                memmove((*sb).storage.as_mut_ptr() as *mut c_void, str_ as *const c_void, len);
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
                let sb = realloc(
                    null_mut(),
                    size_of::<StbdsStringBlock>() - 8 + blocksize,
                ) as *mut StbdsStringBlock;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }
        let p = (*(*a).storage)
            .storage
            .as_mut_ptr()
            .add((*a).remaining - len);
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
    const FMT: &[u8] = b"test_%d\0";
    unsafe {
        sprintf(
            core::ptr::addr_of_mut!(BUFFER) as *mut c_char,
            FMT.as_ptr() as *const c_char,
            n,
        );
        core::ptr::addr_of_mut!(BUFFER) as *mut c_char
    }
}

unsafe fn c_assert_intput(ok: bool, expr: &'static [u8], line: c_int) {
    if !ok {
        const FILE: &[u8] = b"c_src/src/lib.c\0";
        const FUNC: &[u8] = b"intput\0";
        unsafe {
            __assert_fail(
                expr.as_ptr() as *const c_char,
                FILE.as_ptr() as *const c_char,
                line,
                FUNC.as_ptr() as *const c_char,
            );
        }
    }
}

unsafe fn hmput_int(mut map: *mut IntEntry, key: c_int, value: c_int) -> *mut IntEntry {
    unsafe {
        let mut k = key;
        map = stbds_hmput_key(
            map as *mut c_void,
            size_of::<IntEntry>(),
            &mut k as *mut c_int as *mut c_void,
            size_of::<c_int>(),
            0,
        ) as *mut IntEntry;
        let raw = hash_to_arr(map as *mut c_void, size_of::<IntEntry>());
        let idx = (*header(raw)).temp;
        (*map.offset(idx)).key = key;
        (*map.offset(idx)).value = value;
        map
    }
}

unsafe fn hmget_int(mut map: *mut IntEntry, key: c_int) -> c_int {
    unsafe {
        let mut k = key;
        map = stbds_hmget_key(
            map as *mut c_void,
            size_of::<IntEntry>(),
            &mut k as *mut c_int as *mut c_void,
            size_of::<c_int>(),
            0,
        ) as *mut IntEntry;
        let raw = hash_to_arr(map as *mut c_void, size_of::<IntEntry>());
        let idx = (*header(raw)).temp;
        (*map.offset(idx)).value
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn intput(num: c_int) {
    unsafe {
        let mut intmap: *mut IntEntry = null_mut();
        intmap = hmput_int(intmap, num, 7);
        intmap = hmput_int(intmap, 11, 3);
        intmap = hmput_int(intmap, 9, num);
        c_assert_intput(
            hmget_int(intmap, 9) == num,
            b"hmget(intmap, 9) == num\0",
            953,
        );
        c_assert_intput(
            hmget_int(intmap, 11) == 3,
            b"hmget(intmap, 11) == 3\0",
            954,
        );
        c_assert_intput(
            hmget_int(intmap, num) == 7,
            b"hmget(intmap, num) == 7\0",
            955,
        );
    }
}
