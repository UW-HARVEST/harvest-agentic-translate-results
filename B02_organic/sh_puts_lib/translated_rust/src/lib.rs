//! Faithful translation of sh_puts C library using stb_ds hash map internals.

use std::alloc::{self, Layout};
use std::ffi::c_int;
use std::ptr;

// ============================================================
// stb_ds constants
// ============================================================
const STBDS_BUCKET_LENGTH: usize = 8;
const STBDS_BUCKET_SHIFT: usize = 3;
const STBDS_BUCKET_MASK: usize = STBDS_BUCKET_LENGTH - 1;
const STBDS_CACHE_LINE_SIZE: usize = 64;

const STBDS_INDEX_EMPTY: isize = -1;
const STBDS_INDEX_DELETED: isize = -2;

const STBDS_HASH_EMPTY: usize = 0;
const STBDS_HASH_DELETED: usize = 1;

const STBDS_HM_STRING: i32 = 1;

const STBDS_SH_ARENA: u8 = 3;

const STBDS_SIZE_T_BITS: u32 = (std::mem::size_of::<usize>() * 8) as u32;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

// ============================================================
// stb_ds structs
// ============================================================
#[repr(C)]
struct StbdsArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut StbdsHashIndex,
    temp: isize,
}

#[repr(C)]
struct StbdsHashBucket {
    hash: [usize; STBDS_BUCKET_LENGTH],
    index: [isize; STBDS_BUCKET_LENGTH],
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

// The strmap entry type: { char *key; int value; }
#[repr(C)]
#[derive(Copy, Clone)]
struct StrMapEntry {
    key: *mut u8,
    value: c_int,
}

// ============================================================
// Helper: raw alloc/realloc/free via libc
// ============================================================
unsafe fn c_realloc(p: *mut u8, size: usize) -> *mut u8 {
    if p.is_null() {
        let layout = Layout::from_size_align_unchecked(size, 8);
        alloc::alloc(layout)
    } else {
        // We don't know old size, use libc realloc directly
        libc::realloc(p as *mut libc::c_void, size) as *mut u8
    }
}

unsafe fn c_free(p: *mut u8) {
    if !p.is_null() {
        libc::free(p as *mut libc::c_void);
    }
}

unsafe fn c_malloc(size: usize) -> *mut u8 {
    libc::malloc(size) as *mut u8
}

// ============================================================
// Array header access
// ============================================================
unsafe fn stbds_header(t: *mut u8) -> *mut StbdsArrayHeader {
    (t as *mut StbdsArrayHeader).offset(-1)
}

unsafe fn stbds_arrlen(a: *mut u8) -> isize {
    if a.is_null() { 0 } else { (*stbds_header(a)).length as isize }
}

unsafe fn stbds_arrcap(a: *mut u8) -> usize {
    if a.is_null() { 0 } else { (*stbds_header(a)).capacity }
}

// ============================================================
// stbds_arrgrowf
// ============================================================
unsafe fn stbds_arrgrowf(a: *mut u8, elemsize: usize, addlen: usize, min_cap_in: usize) -> *mut u8 {
    let mut min_cap = min_cap_in;
    let min_len = (stbds_arrlen(a) as usize) + addlen;

    if min_len > min_cap { min_cap = min_len; }
    if min_cap <= stbds_arrcap(a) { return a; }

    let old_cap = stbds_arrcap(a);
    if min_cap < 2 * old_cap {
        min_cap = 2 * old_cap;
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let alloc_size = elemsize * min_cap + std::mem::size_of::<StbdsArrayHeader>();
    let raw = if a.is_null() {
        c_realloc(ptr::null_mut(), alloc_size)
    } else {
        libc::realloc(stbds_header(a) as *mut libc::c_void, alloc_size) as *mut u8
    };

    let b = raw.add(std::mem::size_of::<StbdsArrayHeader>());
    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;
    b
}

// ============================================================
// Hash functions (must match C exactly for identical behavior)
// ============================================================
fn stbds_rotate_left(val: usize, n: u32) -> usize {
    val.wrapping_shl(n) | val.wrapping_shr(STBDS_SIZE_T_BITS - n)
}

fn stbds_rotate_right(val: usize, n: u32) -> usize {
    val.wrapping_shr(n) | val.wrapping_shl(STBDS_SIZE_T_BITS - n)
}

unsafe fn stbds_hash_string(str_ptr: *const u8, seed: usize) -> usize {
    let mut hash = seed;
    let mut p = str_ptr;
    while *p != 0 {
        hash = stbds_rotate_left(hash, 9).wrapping_add(*p as usize);
        p = p.add(1);
    }
    hash ^= seed;
    hash = (!hash).wrapping_add(hash.wrapping_shl(18));
    hash ^= hash ^ stbds_rotate_right(hash, 31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ stbds_rotate_right(hash, 11);
    hash = hash.wrapping_add(hash.wrapping_shl(6));
    hash ^= stbds_rotate_right(hash, 22);
    hash.wrapping_add(seed)
}

// ============================================================
// Hash index
// ============================================================
fn align_fwd(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}

fn stbds_log2(mut slot_count: usize) -> usize {
    let mut n = 0usize;
    while slot_count > 1 {
        slot_count >>= 1;
        n += 1;
    }
    n
}

fn stbds_probe_position(hash: usize, slot_count: usize, _slot_log2: usize) -> usize {
    hash & (slot_count - 1)
}

static mut STBDS_HASH_SEED: usize = 0x31415926;

unsafe fn stbds_make_hash_index(slot_count: usize, ot: *mut StbdsHashIndex) -> *mut StbdsHashIndex {
    let num_buckets = slot_count >> STBDS_BUCKET_SHIFT;
    let alloc_size = num_buckets * std::mem::size_of::<StbdsHashBucket>()
        + std::mem::size_of::<StbdsHashIndex>()
        + STBDS_CACHE_LINE_SIZE - 1;
    let raw = c_malloc(alloc_size);
    let t = raw as *mut StbdsHashIndex;
    let after_index = (t as usize) + std::mem::size_of::<StbdsHashIndex>();
    (*t).storage = align_fwd(after_index, STBDS_CACHE_LINE_SIZE) as *mut StbdsHashBucket;
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
        (*t).string = ptr::read(&(*ot).string);
        (*t).seed = (*ot).seed;
    } else {
        ptr::write_bytes(&mut (*t).string as *mut StbdsStringArena, 0, 1);
        (*t).seed = STBDS_HASH_SEED;
        // stbds_load_32_or_64 for a
        let v32a: usize = 2147001325;
        let v64_hi_a: usize = 0x27bb2ee6;
        let v64_lo_a: usize = 0x87b0b0fd;
        let mut temp: usize = v64_lo_a ^ v32a;
        temp = temp.wrapping_shl(16).wrapping_shl(16).wrapping_shr(16).wrapping_shr(16);
        let mut a_val: usize = v64_hi_a;
        a_val = a_val.wrapping_shl(16).wrapping_shl(16);
        a_val ^= temp ^ v32a;

        let v32b: usize = 715136305;
        let v64_lo_b: usize = 0xb504f32d;
        let mut temp2: usize = v64_lo_b ^ v32b;
        temp2 = temp2.wrapping_shl(16).wrapping_shl(16).wrapping_shr(16).wrapping_shr(16);
        let mut b_val: usize = 0usize;
        b_val = b_val.wrapping_shl(16).wrapping_shl(16);
        b_val ^= temp2 ^ v32b;

        STBDS_HASH_SEED = STBDS_HASH_SEED.wrapping_mul(a_val).wrapping_add(b_val);
    }

    // Initialize buckets
    for i in 0..num_buckets {
        let bucket = &mut *(*t).storage.add(i);
        for j in 0..STBDS_BUCKET_LENGTH {
            bucket.hash[j] = STBDS_HASH_EMPTY;
            bucket.index[j] = STBDS_INDEX_EMPTY;
        }
    }

    // Rehash from old table
    if !ot.is_null() {
        (*t).used_count = (*ot).used_count;
        let old_num_buckets = (*ot).slot_count >> STBDS_BUCKET_SHIFT;
        for i in 0..old_num_buckets {
            let ob = &*(*ot).storage.add(i);
            for j in 0..STBDS_BUCKET_LENGTH {
                if ob.index[j] >= 0 {
                    let hash = ob.hash[j];
                    let mut pos = stbds_probe_position(hash, (*t).slot_count, (*t).slot_count_log2);
                    let mut step = STBDS_BUCKET_LENGTH;
                    'outer: loop {
                        let bucket = &mut *(*t).storage.add(pos >> STBDS_BUCKET_SHIFT);
                        let start = pos & STBDS_BUCKET_MASK;
                        for z in start..STBDS_BUCKET_LENGTH {
                            if bucket.hash[z] == 0 {
                                bucket.hash[z] = hash;
                                bucket.index[z] = ob.index[j];
                                break 'outer;
                            }
                        }
                        for z in 0..start {
                            if bucket.hash[z] == 0 {
                                bucket.hash[z] = hash;
                                bucket.index[z] = ob.index[j];
                                break 'outer;
                            }
                        }
                        pos = pos.wrapping_add(step) & ((*t).slot_count - 1);
                        step += STBDS_BUCKET_LENGTH;
                    }
                }
            }
        }
    }

    t
}

// ============================================================
// String arena
// ============================================================
unsafe fn stbds_stralloc(a: *mut StbdsStringArena, str_ptr: *const u8) -> *mut u8 {
    let len = libc::strlen(str_ptr as *const libc::c_char) + 1;
    if len > (*a).remaining {
        let blocksize_shift = (*a).block;
        let mut blocksize = (STBDS_STRING_ARENA_BLOCKSIZE_MIN as usize) << (blocksize_shift as usize >> 1);
        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block += 1;
        }
        if len > blocksize {
            let sb_size = std::mem::size_of::<StbdsStringBlock>() - 8 + len;
            let sb = c_malloc(sb_size) as *mut StbdsStringBlock;
            ptr::copy_nonoverlapping(str_ptr, (*sb).storage.as_mut_ptr(), len);
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
            let sb_size = std::mem::size_of::<StbdsStringBlock>() - 8 + blocksize;
            let sb = c_malloc(sb_size) as *mut StbdsStringBlock;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }
    let p = (*(*a).storage).storage.as_mut_ptr().add((*a).remaining - len);
    (*a).remaining -= len;
    ptr::copy_nonoverlapping(str_ptr, p, len);
    p
}

unsafe fn stbds_strreset(a: *mut StbdsStringArena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        c_free(x as *mut u8);
        x = y;
    }
    ptr::write_bytes(a, 0, 1);
}

// ============================================================
// Key comparison for string mode
// ============================================================
unsafe fn stbds_is_key_equal_string(a: *mut u8, elemsize: usize, key: *const u8, i: isize) -> bool {
    let entry_key_ptr = *(a.add(elemsize * (i as usize)) as *const *const u8);
    libc::strcmp(key as *const libc::c_char, entry_key_ptr as *const libc::c_char) == 0
}

// ============================================================
// stbds_hm_find_slot (string mode only)
// ============================================================
unsafe fn stbds_hm_find_slot(a: *mut u8, elemsize: usize, key: *const u8, table: *mut StbdsHashIndex) -> isize {
    let mut hash = stbds_hash_string(key, (*table).seed);
    if hash < 2 { hash += 2; }
    let mut step = STBDS_BUCKET_LENGTH;
    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);

    loop {
        let bucket = &*(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        let start = pos & STBDS_BUCKET_MASK;
        for i in start..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal_string(a, elemsize, key, bucket.index[i]) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if bucket.hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }
        for i in 0..start {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal_string(a, elemsize, key, bucket.index[i]) {
                    return ((pos & !STBDS_BUCKET_MASK) + i) as isize;
                }
            } else if bucket.hash[i] == STBDS_HASH_EMPTY {
                return -1;
            }
        }
        pos = pos.wrapping_add(step) & ((*table).slot_count - 1);
        step += STBDS_BUCKET_LENGTH;
    }
}

// ============================================================
// stbds_hmput_key (string mode, arena)
// ============================================================
unsafe fn stbds_hmput_key_string(a_in: *mut u8, elemsize: usize, key: *const u8) -> *mut u8 {
    let mut a: *mut u8;
    if a_in.is_null() {
        a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        ptr::write_bytes(a, 0, elemsize);
        (*stbds_header(a)).length += 1;
        a = a.add(elemsize); // ARR_TO_HASH
    } else {
        a = a_in;
    }

    let raw_a = a;
    let arr = a.sub(elemsize); // HASH_TO_ARR

    let mut table = (*stbds_header(arr)).hash_table;

    if table.is_null() || (*table).used_count >= (*table).used_count_threshold {
        let slot_count = if table.is_null() { STBDS_BUCKET_LENGTH } else { (*table).slot_count * 2 };
        let nt = stbds_make_hash_index(slot_count, table);
        if !table.is_null() {
            c_free(table as *mut u8);
        } else {
            (*nt).string.mode = STBDS_SH_ARENA; // was STBDS_SH_DEFAULT for HM_STRING but we override with sh_new_arena
        }
        (*stbds_header(arr)).hash_table = nt;
        table = nt;
    }

    let mut hash = stbds_hash_string(key, (*table).seed);
    if hash < 2 { hash += 2; }
    let mut step = STBDS_BUCKET_LENGTH;
    let mut pos = stbds_probe_position(hash, (*table).slot_count, (*table).slot_count_log2);
    let mut tombstone: isize = -1;

    let found_pos: usize;
    'search: loop {
        let bucket = &mut *(*table).storage.add(pos >> STBDS_BUCKET_SHIFT);
        let start = pos & STBDS_BUCKET_MASK;

        for i in start..STBDS_BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal_string(raw_a, elemsize, key, bucket.index[i]) {
                    (*stbds_header(arr)).temp = bucket.index[i];
                    // temp_key
                    let entry_key = *(raw_a.add(elemsize * (bucket.index[i] as usize)) as *const *mut u8);
                    (*table).temp_key = entry_key;
                    return a;
                }
            } else if bucket.hash[i] == 0 {
                found_pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 && bucket.index[i] == STBDS_INDEX_DELETED {
                tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
            }
        }

        for i in 0..start {
            if bucket.hash[i] == hash {
                if stbds_is_key_equal_string(raw_a, elemsize, key, bucket.index[i]) {
                    (*stbds_header(arr)).temp = bucket.index[i];
                    return a;
                }
            } else if bucket.hash[i] == 0 {
                found_pos = (pos & !STBDS_BUCKET_MASK) + i;
                break 'search;
            } else if tombstone < 0 && bucket.index[i] == STBDS_INDEX_DELETED {
                tombstone = ((pos & !STBDS_BUCKET_MASK) + i) as isize;
            }
        }

        pos = pos.wrapping_add(step) & ((*table).slot_count - 1);
        step += STBDS_BUCKET_LENGTH;
    }

    let final_pos = if tombstone >= 0 {
        (*table).tombstone_count -= 1;
        tombstone as usize
    } else {
        found_pos
    };
    (*table).used_count += 1;

    let i = stbds_arrlen(arr);
    let mut arr2 = arr;
    if (i as usize) + 1 > stbds_arrcap(arr) {
        arr2 = stbds_arrgrowf(arr, elemsize, 1, 0);
    }
    let raw_a2 = arr2.add(elemsize);

    (*stbds_header(arr2)).length = (i + 1) as usize;
    let bucket = &mut *(*table).storage.add(final_pos >> STBDS_BUCKET_SHIFT);
    bucket.hash[final_pos & STBDS_BUCKET_MASK] = hash;
    bucket.index[final_pos & STBDS_BUCKET_MASK] = i - 1;
    (*stbds_header(arr2)).temp = i - 1;

    // Arena mode: copy key
    let new_key = stbds_stralloc(&mut (*table).string, key);
    *(raw_a2.add(elemsize * (i as usize)) as *mut *mut u8) = new_key;
    (*table).temp_key = new_key;

    raw_a2
}

// ============================================================
// stbds_shmode_func (arena mode)
// ============================================================
unsafe fn stbds_shmode_func(elemsize: usize, mode: u8) -> *mut u8 {
    let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
    ptr::write_bytes(a, 0, elemsize);
    (*stbds_header(a)).length = 1;
    let h = stbds_make_hash_index(STBDS_BUCKET_LENGTH, ptr::null_mut());
    (*h).string.mode = mode;
    (*stbds_header(a)).hash_table = h;
    a.add(elemsize) // ARR_TO_HASH
}

// ============================================================
// stbds_hmfree_func
// ============================================================
unsafe fn stbds_hmfree_func(a: *mut u8, elemsize: usize) {
    if a.is_null() { return; }
    let arr = a.sub(elemsize);
    let table = (*stbds_header(arr)).hash_table;
    if !table.is_null() {
        stbds_strreset(&mut (*table).string);
        c_free(table as *mut u8);
    }
    c_free(stbds_header(arr) as *mut u8);
}

// ============================================================
// strkey helper (matches C: sprintf(buffer, "test_%d", n))
// ============================================================
unsafe fn strkey(n: c_int, buffer: &mut [u8; 256]) -> *const u8 {
    let s = format!("test_{}", n);
    let bytes = s.as_bytes();
    ptr::copy_nonoverlapping(bytes.as_ptr(), buffer.as_mut_ptr(), bytes.len());
    buffer[bytes.len()] = 0;
    buffer.as_ptr()
}

// ============================================================
// sh_puts - the public API
// ============================================================
/// # Safety
/// This is an extern "C" function matching the C library's public API.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sh_puts(num: c_int) {
    let elemsize = std::mem::size_of::<StrMapEntry>();
    let mut buffer = [0u8; 256];

    // Phase 1: stralloc/strreset loop (matches C exactly)
    let mut sa = StbdsStringArena {
        storage: ptr::null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };
    for i in 0..num {
        let k = strkey(i, &mut buffer);
        stbds_stralloc(&mut sa, k);
    }
    stbds_strreset(&mut sa);

    // Phase 2: string hash map operations
    // sh_new_arena(strmap)
    let mut strmap: *mut u8 = stbds_shmode_func(elemsize, STBDS_SH_ARENA);

    // s.key = "a", s.value = num
    // shputs(strmap, s) expands to:
    //   strmap = stbds_hmput_key_wrapper(strmap, sizeof *strmap, (void*)(s).key, sizeof(s).key, STBDS_HM_STRING)
    //   strmap[temp] = s
    //   strmap[temp].key = stbds_temp_key(strmap-1)
    let key_a = b"a\0";
    strmap = stbds_hmput_key_string(strmap, elemsize, key_a.as_ptr());
    let temp = (*stbds_header(strmap.sub(elemsize))).temp;
    let entry = &mut *(strmap.add(elemsize * (temp as usize)) as *mut StrMapEntry);
    // s.key = "a", s.value = num -> strmap[temp] = s
    entry.value = num;
    // strmap[temp].key = stbds_temp_key(strmap-1)
    let table = (*stbds_header(strmap.sub(elemsize))).hash_table;
    entry.key = (*table).temp_key;

    // Assertions (matching C asserts)
    assert_eq!(*entry.key, b'a');
    assert_ne!(entry.key, key_a.as_ptr() as *mut u8);
    assert_eq!(entry.value, num);

    // shlen(strmap) = header(strmap-1)->length - 1
    let arr = strmap.sub(elemsize);
    let map_len = (*stbds_header(arr)).length as isize - 1;

    // for (int z=0; z < shlen(strmap); ++z)
    //     printf("%s %d\n", strmap[z], strmap[z].value);
    // Note: strmap[z] as %s prints the key (first field of struct)
    for z in 0..map_len {
        let e = &*(strmap.add(elemsize * (z as usize)) as *const StrMapEntry);
        libc::printf(
            b"%s %d\n\0".as_ptr() as *const libc::c_char,
            e.key as *const libc::c_char,
            e.value as c_int,
        );
    }

    // shfree(strmap)
    stbds_hmfree_func(strmap, elemsize);
}
