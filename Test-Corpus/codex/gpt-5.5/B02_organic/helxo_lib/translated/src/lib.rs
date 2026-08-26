use std::ffi::{c_char, c_int, c_void};
use std::mem;
use std::ptr;

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
const STBDS_SH_DEFAULT: u8 = 1;
const STBDS_SH_STRDUP: u8 = 2;
const STBDS_SH_ARENA: u8 = 3;
const STBDS_STRING_ARENA_BLOCKSIZE_MIN: SizeT = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: SizeT = 1 << 20;

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
#[derive(Copy, Clone)]
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

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: SizeT) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memset(ptr: *mut c_void, value: c_int, num: SizeT) -> *mut c_void;
    fn memmove(dst: *mut c_void, src: *const c_void, num: SizeT) -> *mut c_void;
    fn memcmp(a: *const c_void, b: *const c_void, num: SizeT) -> c_int;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn strlen(s: *const c_char) -> SizeT;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: SizeT, fmt: *const c_char, ...) -> c_int;
}

static mut STBDS_HASH_SEED: SizeT = 0x31415926;
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

#[inline]
unsafe fn header(t: *mut c_void) -> *mut StbdsArrayHeader {
    unsafe { (t as *mut StbdsArrayHeader).sub(1) }
}

#[inline]
unsafe fn arr_len(a: *mut c_void) -> SizeT {
    if a.is_null() {
        0
    } else {
        unsafe { (*header(a)).length }
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
unsafe fn hash_to_arr(a: *mut c_void, elemsize: SizeT) -> *mut c_void {
    unsafe { (a as *mut u8).sub(elemsize) as *mut c_void }
}

#[inline]
unsafe fn arr_to_hash(a: *mut c_void, elemsize: SizeT) -> *mut c_void {
    unsafe { (a as *mut u8).add(elemsize) as *mut c_void }
}

#[inline]
unsafe fn hash_table(a: *mut c_void) -> *mut StbdsHashIndex {
    unsafe { (*header(a)).hash_table as *mut StbdsHashIndex }
}

#[inline]
fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

#[inline]
fn probe_position(hash: SizeT, slot_count: SizeT, _slot_log2: SizeT) -> SizeT {
    hash & (slot_count - 1)
}

#[inline]
fn index_in_use(x: PtrDiffT) -> bool {
    x >= 0
}

fn stbds_log2(mut slot_count: SizeT) -> SizeT {
    let mut n = 0;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: SizeT,
    addlen: SizeT,
    mut min_cap: SizeT,
) -> *mut c_void {
    let min_len = unsafe { arr_len(a) }.wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= unsafe { arr_cap(a) } {
        return a;
    }

    let cap = unsafe { arr_cap(a) };
    if min_cap < 2usize.wrapping_mul(cap) {
        min_cap = 2usize.wrapping_mul(cap);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old = if a.is_null() {
        ptr::null_mut()
    } else {
        unsafe { header(a) as *mut c_void }
    };
    let total = elemsize
        .wrapping_mul(min_cap)
        .wrapping_add(mem::size_of::<StbdsArrayHeader>());
    let mut b = unsafe { realloc(old, total) };
    b = unsafe { (b as *mut u8).add(mem::size_of::<StbdsArrayHeader>()) as *mut c_void };

    if a.is_null() {
        unsafe {
            (*header(b)).length = 0;
            (*header(b)).hash_table = ptr::null_mut();
            (*header(b)).temp = 0;
        }
    }
    unsafe {
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
    unsafe {
        STBDS_HASH_SEED = seed;
    }
}

unsafe fn make_hash_index(slot_count: SizeT, ot: *mut StbdsHashIndex) -> *mut StbdsHashIndex {
    let bytes = (slot_count >> STBDS_BUCKET_SHIFT)
        .wrapping_mul(mem::size_of::<StbdsHashBucket>())
        .wrapping_add(mem::size_of::<StbdsHashIndex>())
        .wrapping_add(STBDS_CACHE_LINE_SIZE - 1);
    let t = unsafe { realloc(ptr::null_mut(), bytes) as *mut StbdsHashIndex };
    unsafe {
        (*t).storage = align_fwd(
            (t as usize).wrapping_add(mem::size_of::<StbdsHashIndex>()),
            STBDS_CACHE_LINE_SIZE,
        ) as *mut StbdsHashBucket;
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
            memset(
                ptr::addr_of_mut!((*t).string) as *mut c_void,
                0,
                mem::size_of::<StbdsStringArena>(),
            );
            (*t).seed = STBDS_HASH_SEED;
            STBDS_HASH_SEED = STBDS_HASH_SEED
                .wrapping_mul(0x27bb_2ee6_87b0_b0fdusize)
                .wrapping_add(0xb504_f32dusize);
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
                    if index_in_use((*ob).index[j]) {
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
    }
    t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(str_: *mut c_char, seed: SizeT) -> SizeT {
    let mut hash = seed;
    let mut p = str_ as *mut u8;
    unsafe {
        while *p != 0 {
            hash = hash.rotate_left(9).wrapping_add(*p as SizeT);
            p = p.add(1);
        }
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

unsafe fn siphash_bytes(p: *mut c_void, len: SizeT, seed: SizeT) -> SizeT {
    let mut d = p as *mut u8;
    let mut v0 = 0x736f_6d65_7073_6575usize ^ seed;
    let mut v1 = 0x646f_7261_6e64_6f6dusize ^ !seed;
    let mut v2 = 0x6c79_6765_6e65_7261usize ^ seed;
    let mut v3 = 0x7465_6462_7974_6573usize ^ !seed;
    v0 ^= 0x0706_0504_0302_0100usize ^ seed;
    v1 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
    v2 ^= 0x0706_0504_0302_0100usize ^ seed;
    v3 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

    macro_rules! sipround {
        () => {{
            v0 = v0.wrapping_add(v1);
            v1 = v1.rotate_left(13);
            v1 ^= v0;
            v0 = v0.rotate_left(SizeT::BITS / 2);
            v2 = v2.wrapping_add(v3);
            v3 = v3.rotate_left(16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = v1.rotate_left(17);
            v1 ^= v2;
            v2 = v2.rotate_left(SizeT::BITS / 2);
            v0 = v0.wrapping_add(v3);
            v3 = v3.rotate_left(21);
            v3 ^= v0;
        }};
    }

    let mut i = 0usize;
    while i + mem::size_of::<SizeT>() <= len {
        let mut data = unsafe {
            (*d.add(0) as SizeT)
                | ((*d.add(1) as SizeT) << 8)
                | ((*d.add(2) as SizeT) << 16)
                | ((*d.add(3) as SizeT) << 24)
        };
        data |= unsafe {
            ((*d.add(4) as SizeT)
                | ((*d.add(5) as SizeT) << 8)
                | ((*d.add(6) as SizeT) << 16)
                | ((*d.add(7) as SizeT) << 24))
                << 16
                << 16
        };
        v3 ^= data;
        for _ in 0..2 {
            sipround!();
        }
        v0 ^= data;
        i += mem::size_of::<SizeT>();
        d = unsafe { d.add(mem::size_of::<SizeT>()) };
    }

    let mut data = len << (SizeT::BITS as usize - 8);
    unsafe {
        match len - i {
            7 => {
                data |= (*d.add(6) as SizeT) << 24 << 24;
                data |= (*d.add(5) as SizeT) << 20 << 20;
                data |= (*d.add(4) as SizeT) << 16 << 16;
                data |= (*d.add(3) as SizeT) << 24;
                data |= (*d.add(2) as SizeT) << 16;
                data |= (*d.add(1) as SizeT) << 8;
                data |= *d.add(0) as SizeT;
            }
            6 => {
                data |= (*d.add(5) as SizeT) << 20 << 20;
                data |= (*d.add(4) as SizeT) << 16 << 16;
                data |= (*d.add(3) as SizeT) << 24;
                data |= (*d.add(2) as SizeT) << 16;
                data |= (*d.add(1) as SizeT) << 8;
                data |= *d.add(0) as SizeT;
            }
            5 => {
                data |= (*d.add(4) as SizeT) << 16 << 16;
                data |= (*d.add(3) as SizeT) << 24;
                data |= (*d.add(2) as SizeT) << 16;
                data |= (*d.add(1) as SizeT) << 8;
                data |= *d.add(0) as SizeT;
            }
            4 => {
                data |= (*d.add(3) as SizeT) << 24;
                data |= (*d.add(2) as SizeT) << 16;
                data |= (*d.add(1) as SizeT) << 8;
                data |= *d.add(0) as SizeT;
            }
            3 => {
                data |= (*d.add(2) as SizeT) << 16;
                data |= (*d.add(1) as SizeT) << 8;
                data |= *d.add(0) as SizeT;
            }
            2 => {
                data |= (*d.add(1) as SizeT) << 8;
                data |= *d.add(0) as SizeT;
            }
            1 => data |= *d.add(0) as SizeT,
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
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: SizeT, seed: SizeT) -> SizeT {
    unsafe { siphash_bytes(p, len, seed) }
}

unsafe fn is_key_equal(
    a: *mut c_void,
    elemsize: SizeT,
    key: *mut c_void,
    keysize: SizeT,
    keyoffset: SizeT,
    mode: c_int,
    i: SizeT,
) -> bool {
    unsafe {
        if mode >= STBDS_HM_STRING {
            let stored = *((a as *mut u8).add(elemsize * i + keyoffset) as *mut *mut c_char);
            strcmp(key as *mut c_char, stored) == 0
        } else {
            memcmp(key, (a as *mut u8).add(elemsize * i + keyoffset) as *mut c_void, keysize) == 0
        }
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
                    free(*((a as *mut u8).add(elemsize * i) as *mut *mut c_void));
                }
            }
            stbds_strreset(ptr::addr_of_mut!((*table).string));
        }
        free((*header(a)).hash_table);
        free(header(a) as *mut c_void);
    }
}

unsafe fn hm_find_slot(
    a: *mut c_void,
    elemsize: SizeT,
    key: *mut c_void,
    keysize: SizeT,
    keyoffset: SizeT,
    mode: c_int,
) -> PtrDiffT {
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
                    if is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as SizeT) {
                        return ((pos & !STBDS_BUCKET_MASK) + i) as PtrDiffT;
                    }
                } else if (*bucket).hash[i] == STBDS_HASH_EMPTY {
                    return -1;
                }
            }
            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as SizeT) {
                        return ((pos & !STBDS_BUCKET_MASK) + i) as PtrDiffT;
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
    elemsize: SizeT,
    key: *mut c_void,
    keysize: SizeT,
    temp: *mut PtrDiffT,
    mode: c_int,
) -> *mut c_void {
    let keyoffset = 0;
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(
    a: *mut c_void,
    elemsize: SizeT,
    key: *mut c_void,
    keysize: SizeT,
    mode: c_int,
) -> *mut c_void {
    let mut temp = 0isize;
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
                if a.is_null() { ptr::null_mut() } else { hash_to_arr(a, elemsize) },
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
        let p = realloc(ptr::null_mut(), len) as *mut c_char;
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
            a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            memset(a, 0, elemsize);
            (*header(a)).length += 1;
            a = arr_to_hash(a, elemsize);
        }

        let raw_a = a;
        a = hash_to_arr(a, elemsize);
        let mut table = hash_table(a);
        if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
            let slot_count = if table.is_null() { STBDS_BUCKET_LENGTH } else { (*table).slot_count * 2 };
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
        let mut bucket: *mut StbdsHashBucket;

        'search: loop {
            bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
            for i in (pos & STBDS_BUCKET_MASK)..STBDS_BUCKET_LENGTH {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as usize) {
                        (*header(a)).temp = (*bucket).index[i];
                        if mode >= STBDS_HM_STRING {
                            (*table).temp_key =
                                *((raw_a as *mut u8).add(elemsize * ((*bucket).index[i] as usize) + keyoffset)
                                    as *mut *mut c_char);
                        }
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'search;
                } else if tombstone < 0 && (*bucket).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as PtrDiffT;
                }
            }
            let limit = pos & STBDS_BUCKET_MASK;
            for i in 0..limit {
                if (*bucket).hash[i] == hash {
                    if is_key_equal(raw_a, elemsize, key, keysize, keyoffset, mode, (*bucket).index[i] as usize) {
                        (*header(a)).temp = (*bucket).index[i];
                        return arr_to_hash(a, elemsize);
                    }
                } else if (*bucket).hash[i] == 0 {
                    pos = (pos & !STBDS_BUCKET_MASK) + i;
                    break 'search;
                } else if tombstone < 0 && (*bucket).index[i] == STBDS_INDEX_DELETED {
                    tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as PtrDiffT;
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

        let i = arr_len(a) as PtrDiffT;
        if (i as usize) + 1 > arr_cap(a) {
            a = stbds_arrgrowf(a, elemsize, 1, 0);
        }
        (*header(a)).length = (i as usize) + 1;
        bucket = (*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        (*bucket).hash[pos & STBDS_BUCKET_MASK] = hash;
        (*bucket).index[pos & STBDS_BUCKET_MASK] = i - 1;
        (*header(a)).temp = i - 1;

        let row = (a as *mut u8).add(elemsize * (i as usize));
        match (*table).string.mode {
            STBDS_SH_STRDUP => {
                let s = stbds_strdup(key as *mut c_char);
                *((row) as *mut *mut c_char) = s;
                (*table).temp_key = s;
            }
            STBDS_SH_ARENA => {
                let s = stbds_stralloc(ptr::addr_of_mut!((*table).string), key as *mut c_char);
                *((row) as *mut *mut c_char) = s;
                (*table).temp_key = s;
            }
            STBDS_SH_DEFAULT => {
                *((row) as *mut *mut c_char) = key as *mut c_char;
                (*table).temp_key = key as *mut c_char;
            }
            _ => {
                ptr::copy(key as *const u8, row, keysize);
            }
        }
        arr_to_hash(a, elemsize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: SizeT, mode: c_int) -> *mut c_void {
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
    elemsize: SizeT,
    key: *mut c_void,
    keysize: SizeT,
    keyoffset: SizeT,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        if a.is_null() {
            ptr::null_mut()
        } else {
            let raw_a = hash_to_arr(a, elemsize);
            let table = hash_table(raw_a);
            (*header(raw_a)).temp = 0;
            if table.is_null() {
                a
            } else {
                let mut slot = hm_find_slot(a, elemsize, key, keysize, keyoffset, mode);
                if slot < 0 {
                    a
                } else {
                    let mut b = (*table).storage.add((slot as usize) >> STBDS_BUCKET_SHIFT);
                    let mut i = (slot as usize) & STBDS_BUCKET_MASK;
                    let old_index = (*b).index[i];
                    let final_index = arr_len(raw_a) as PtrDiffT - 1 - 1;
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
                        slot = if mode == STBDS_HM_STRING {
                            hm_find_slot(
                                a,
                                elemsize,
                                *((a as *mut u8).add(elemsize * (old_index as usize) + keyoffset)
                                    as *mut *mut c_void),
                                keysize,
                                keyoffset,
                                mode,
                            )
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
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut StbdsStringArena, str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(str_) + 1;
        if len > (*a).remaining {
            let mut blocksize = (*a).block as SizeT;
            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);
            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }
            if len > blocksize {
                let sb = realloc(
                    ptr::null_mut(),
                    mem::size_of::<StbdsStringBlock>() - 8 + len,
                ) as *mut StbdsStringBlock;
                memmove(
                    ptr::addr_of_mut!((*sb).storage) as *mut c_void,
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
                return ptr::addr_of_mut!((*sb).storage) as *mut c_char;
            } else {
                let sb = realloc(
                    ptr::null_mut(),
                    mem::size_of::<StbdsStringBlock>() - 8 + blocksize,
                ) as *mut StbdsStringBlock;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }
        let p = (ptr::addr_of_mut!((*(*a).storage).storage) as *mut c_char).add((*a).remaining - len);
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
        memset(a as *mut c_void, 0, mem::size_of::<StbdsStringArena>());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    static FMT: &[u8] = b"test_%d\0";
    unsafe {
        snprintf(
            ptr::addr_of_mut!(STRKEY_BUFFER) as *mut c_char,
            256,
            FMT.as_ptr() as *const c_char,
            n,
        );
        ptr::addr_of_mut!(STRKEY_BUFFER) as *mut c_char
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn helxo(letter: c_char) {
    static FMT: &[u8] = b"%s %c\n\0";
    static BOB: &[u8] = b"bob\0";
    static SALLY: &[u8] = b"sally\0";
    static FRED: &[u8] = b"fred\0";
    static JEN: &[u8] = b"jen\0";
    static DOUG: &[u8] = b"doug\0";
    unsafe {
        printf(FMT.as_ptr() as *const c_char, BOB.as_ptr() as *const c_char, b'h' as c_int);
        printf(FMT.as_ptr() as *const c_char, SALLY.as_ptr() as *const c_char, b'e' as c_int);
        printf(FMT.as_ptr() as *const c_char, FRED.as_ptr() as *const c_char, b'l' as c_int);
        printf(FMT.as_ptr() as *const c_char, JEN.as_ptr() as *const c_char, letter as c_int);
        printf(FMT.as_ptr() as *const c_char, DOUG.as_ptr() as *const c_char, b'o' as c_int);
    }
}
