use std::alloc::{alloc, dealloc, realloc, Layout};
use std::mem::{align_of, size_of};
use std::ptr::{self, NonNull};
use std::slice;

const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

static mut STBDS_HASH_SEED: usize = 0x31415926;

#[repr(C)]
struct StbdsArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut StbdsHashIndex,
    temp: isize,
}

#[repr(C)]
struct StbdsStringBlock {
    next: *mut StbdsStringBlock,
    storage: [u8; 8],
}

#[repr(C)]
struct StbdsStringArena {
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
    temp_key: *mut u8,
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

unsafe fn stbds_header<T>(arr: *mut T) -> *mut StbdsArrayHeader {
    (arr as *mut StbdsArrayHeader).offset(-1)
}

unsafe fn stbds_arrlen<T>(arr: *mut T) -> usize {
    if arr.is_null() {
        0
    } else {
        (*stbds_header(arr)).length
    }
}

unsafe fn stbds_arrcap<T>(arr: *mut T) -> usize {
    if arr.is_null() {
        0
    } else {
        (*stbds_header(arr)).capacity
    }
}

unsafe fn stbds_arrgrowf<T>(arr: *mut T, addlen: usize, min_cap: usize) -> *mut T {
    let elemsize = size_of::<T>();
    let min_len = stbds_arrlen(arr) + addlen;
    let mut min_cap = if min_len > min_cap { min_len } else { min_cap };

    if min_cap <= stbds_arrcap(arr) {
        return arr;
    }

    if min_cap < 2 * stbds_arrcap(arr) {
        min_cap = 2 * stbds_arrcap(arr);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let header_size = size_of::<StbdsArrayHeader>();
    let new_size = header_size + elemsize * min_cap;

    let new_ptr = if arr.is_null() {
        let layout = Layout::from_size_align(new_size, align_of::<T>()).unwrap();
        alloc(layout)
    } else {
        let old_header = stbds_header(arr);
        let old_cap = (*old_header).capacity;
        let old_size = header_size + elemsize * old_cap;
        let old_layout = Layout::from_size_align(old_size, align_of::<T>()).unwrap();
        realloc(old_header as *mut u8, old_layout, new_size)
    };

    let new_header = new_ptr as *mut StbdsArrayHeader;
    if arr.is_null() {
        (*new_header).length = 0;
        (*new_header).hash_table = ptr::null_mut();
        (*new_header).temp = 0;
    }
    (*new_header).capacity = min_cap;

    (new_header.offset(1)) as *mut T
}

unsafe fn stbds_arrmaybegrow<T>(arr: *mut T, n: usize) -> *mut T {
    if arr.is_null() || (*stbds_header(arr)).length + n > (*stbds_header(arr)).capacity {
        stbds_arrgrowf(arr, n, 0)
    } else {
        arr
    }
}

unsafe fn stbds_rotate_left(val: usize, n: usize) -> usize {
    let bits = size_of::<usize>() * 8;
    (val << n) | (val >> (bits - n))
}

unsafe fn stbds_rotate_right(val: usize, n: usize) -> usize {
    let bits = size_of::<usize>() * 8;
    (val >> n) | (val << (bits - n))
}

unsafe fn stbds_hash_string(str: *const u8, seed: usize) -> usize {
    let mut hash = seed;
    let mut p = str;
    while *p != 0 {
        hash = stbds_rotate_left(hash, 9) + (*p as usize);
        p = p.offset(1);
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

unsafe fn stbds_siphash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    let mut v0: usize = 0x736f6d6570736575 ^ seed;
    let mut v1: usize = 0x646f72616e646f6d ^ !seed;
    let mut v2: usize = 0x6c7967656e657261 ^ seed;
    let mut v3: usize = 0x7465646279746573 ^ !seed;

    v0 ^= 0x0706050403020100 ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908 ^ !seed;
    v2 ^= 0x0706050403020100 ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908 ^ !seed;

    let mut d = p;
    let mut i = 0;

    macro_rules! sipround {
        () => {
            v0 = v0.wrapping_add(v1);
            v1 = stbds_rotate_left(v1, 13);
            v1 ^= v0;
            v0 = stbds_rotate_left(v0, size_of::<usize>() * 4);
            v2 = v2.wrapping_add(v3);
            v3 = stbds_rotate_left(v3, 16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = stbds_rotate_left(v1, 17);
            v1 ^= v2;
            v2 = stbds_rotate_left(v2, size_of::<usize>() * 4);
            v0 = v0.wrapping_add(v3);
            v3 = stbds_rotate_left(v3, 21);
            v3 ^= v0;
        };
    }

    while i + size_of::<usize>() <= len {
        let mut data: usize = 0;
        ptr::copy_nonoverlapping(d, &mut data as *mut usize as *mut u8, size_of::<usize>());

        v3 ^= data;
        for _ in 0..2 {
            sipround!();
        }
        v0 ^= data;

        i += size_of::<usize>();
        d = d.add(size_of::<usize>());
    }

    let mut data: usize = len << (size_of::<usize>() * 8 - 8);
    let remaining = len - i;
    match remaining {
        7 => data |= ((*d.add(6) as usize) << 48),
        6 => data |= ((*d.add(5) as usize) << 40),
        5 => data |= ((*d.add(4) as usize) << 32),
        4 => data |= ((*d.add(3) as usize) << 24),
        3 => data |= ((*d.add(2) as usize) << 16),
        2 => data |= ((*d.add(1) as usize) << 8),
        1 => data |= (*d as usize),
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

unsafe fn stbds_hash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

unsafe fn stbds_log2(slot_count: usize) -> usize {
    let mut n = 0;
    let mut sc = slot_count;
    while sc > 1 {
        sc >>= 1;
        n += 1;
    }
    n
}

unsafe fn stbds_make_hash_index(slot_count: usize, ot: *mut StbdsHashIndex) -> *mut StbdsHashIndex {
    let bucket_count = slot_count >> STBDS_BUCKET_SHIFT;
    let storage_size = bucket_count * size_of::<StbdsHashBucket>();
    let total_size = size_of::<StbdsHashIndex>() + storage_size + STBDS_CACHE_LINE_SIZE - 1;

    let layout = Layout::from_size_align(total_size, align_of::<StbdsHashIndex>()).unwrap();
    let t = alloc(layout) as *mut StbdsHashIndex;

    let storage_offset = size_of::<StbdsHashIndex>();
    let aligned_storage = ((t as usize) + storage_offset + STBDS_CACHE_LINE_SIZE - 1) & !(STBDS_CACHE_LINE_SIZE - 1);
    (*t).storage = aligned_storage as *mut StbdsHashBucket;
    (*t).slot_count = slot_count;
    (*t).slot_count_log2 = stbds_log2(slot_count);
    (*t).tombstone_count = 0;
    (*t).used_count = 0;

    (*t).used_count_threshold = slot_count - (slot_count >> 2);
    (*t).tombstone_count_threshold = (slot_count >> 3) + (slot_count >> 4);
    (*t).used_count_shrink_threshold = if slot_count <= STBDS_BUCKET_LENGTH { 0 } else { slot_count >> 2 };

    if !ot.is_null() {
        (*t).string = (*ot).string;
        (*t).seed = (*ot).seed;
    } else {
        ptr::write_bytes(&mut (*t).string as *mut StbdsStringArena, 0, 1);
        (*t).seed = STBDS_HASH_SEED;
        let a: usize = 0x27bb2ee687b0b0fd ^ 0x87b0b0fd;
        let b: usize = 0xb504f32d;
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
                let idx = (*ob).index[j];
                if idx >= 0 {
                    let hash = (*ob).hash[j];
                    let mut pos = hash & (slot_count - 1);
                    let mut step = STBDS_BUCKET_LENGTH;

                    'outer: loop {
                        let bucket = (*t).storage.add(pos >> STBDS_BUCKET_SHIFT);
                        let start = pos & STBDS_BUCKET_MASK;

                        for z in start..STBDS_BUCKET_LENGTH {
                            if (*bucket).hash[z] == STBDS_HASH_EMPTY {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = idx;
                                break 'outer;
                            }
                        }

                        for z in 0..start {
                            if (*bucket).hash[z] == STBDS_HASH_EMPTY {
                                (*bucket).hash[z] = hash;
                                (*bucket).index[z] = idx;
                                break 'outer;
                            }
                        }

                        pos = (pos + step) & (slot_count - 1);
                        step += STBDS_BUCKET_LENGTH;
                    }
                }
            }
        }
    }

    t
}

unsafe fn stbds_probe_position(hash: usize, slot_count: usize) -> usize {
    hash & (slot_count - 1)
}

unsafe fn stbds_is_key_equal(a: *const u8, elemsize: usize, key: *const u8, keysize: usize, keyoffset: usize, mode: i32, i: isize) -> bool {
    if mode >= 1 {
        let str_a = *(a.add(elemsize * i as usize + keyoffset) as *const *const u8);
        libc::strcmp(key as *const i8, str_a as *const i8) == 0
    } else {
        libc::memcmp(key as *const libc::c_void, a.add(elemsize * i as usize + keyoffset) as *const libc::c_void, keysize) == 0
    }
}

unsafe fn stbds_hm_find_slot(a: *const u8, elemsize: usize, key: *const u8, keysize: usize, keyoffset: usize, mode: i32, table: *mut StbdsHashIndex) -> isize {
    let hash = if mode >= 1 {
        stbds_hash_string(key, (*table).seed)
    } else {
        stbds_hash_bytes(key, keysize, (*table).seed)
    };
    let hash = if hash < 2 { hash + 2 } else { hash };

    let mut step = STBDS_BUCKET_LENGTH;
    let mut pos = stbds_probe_position(hash, (*table).slot_count);

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

        pos = (pos + step) & ((*table).slot_count - 1);
        step += STBDS_BUCKET_LENGTH;
    }
}

unsafe fn stbds_stralloc(a: *mut StbdsStringArena, str: *const u8) -> *mut u8 {
    let len = libc::strlen(str as *const i8) + 1;

    if len > (*a).remaining {
        let mut blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << ((*a).block >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }

        if len > blocksize {
            let sb_size = size_of::<StbdsStringBlock>() - 8 + len;
            let layout = Layout::from_size_align(sb_size, align_of::<StbdsStringBlock>()).unwrap();
            let sb = alloc(layout) as *mut StbdsStringBlock;
            ptr::copy_nonoverlapping(str, (*sb).storage.as_mut_ptr(), len);

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
            let sb_size = size_of::<StbdsStringBlock>() - 8 + blocksize;
            let layout = Layout::from_size_align(sb_size, align_of::<StbdsStringBlock>()).unwrap();
            let sb = alloc(layout) as *mut StbdsStringBlock;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    let p = (*(*a).storage).storage.as_mut_ptr().add((*a).remaining - len);
    (*a).remaining -= len;
    ptr::copy_nonoverlapping(str, p, len);
    p
}

unsafe fn stbds_strreset(a: *mut StbdsStringArena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        let layout = Layout::from_size_align(size_of::<StbdsStringBlock>(), align_of::<StbdsStringBlock>()).unwrap();
        dealloc(x as *mut u8, layout);
        x = y;
    }
    ptr::write_bytes(a, 0, 1);
}

unsafe fn stbds_strdup(str: *const u8) -> *mut u8 {
    let len = libc::strlen(str as *const i8) + 1;
    let layout = Layout::from_size_align(len, 1).unwrap();
    let p = alloc(layout);
    ptr::copy_nonoverlapping(str, p, len);
    p
}

static mut ARR: *mut i32 = ptr::null_mut();

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arr_push(num: i32) {
    let num = num as usize;
    ARR = ptr::null_mut();

    assert!(stbds_arrlen(ARR) == 0);

    for i in (0..num).step_by(50) {
        for j in 0..i {
            ARR = stbds_arrmaybegrow(ARR, 1);
            let len = (*stbds_header(ARR)).length;
            *ARR.add(len) = j as i32;
            (*stbds_header(ARR)).length = len + 1;
        }

        if !ARR.is_null() {
            let header = stbds_header(ARR);
            let layout = Layout::from_size_align(
                size_of::<StbdsArrayHeader>() + (*header).capacity * size_of::<i32>(),
                align_of::<i32>()
            ).unwrap();
            dealloc(header as *mut u8, layout);
            ARR = ptr::null_mut();
        }
    }
}
