#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr::{addr_of_mut, null_mut};

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
pub struct StbdsStringArena {
    storage: *mut StbdsStringBlock,
    remaining: usize,
    block: u8,
    mode: u8,
}

static mut STBDS_HASH_SEED: usize = 0x3141_5926;
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(dest: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

unsafe fn header_from_array(a: *mut c_void) -> *mut StbdsArrayHeader {
    (a as *mut u8).sub(size_of::<StbdsArrayHeader>()) as *mut StbdsArrayHeader
}

unsafe fn arr_to_hash(a: *mut c_void, elemsize: usize) -> *mut c_void {
    (a as *mut u8).add(elemsize) as *mut c_void
}

unsafe fn hash_to_arr(a: *mut c_void, elemsize: usize) -> *mut c_void {
    (a as *mut u8).sub(elemsize) as *mut c_void
}

unsafe fn arr_len(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*header_from_array(a)).length
    }
}

unsafe fn arr_cap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        (*header_from_array(a)).capacity
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    let min_len = arr_len(a).wrapping_add(addlen);
    if min_len > min_cap {
        min_cap = min_len;
    }

    let old_cap = arr_cap(a);
    if min_cap <= old_cap {
        return a;
    }

    if min_cap < old_cap.wrapping_mul(2) {
        min_cap = old_cap.wrapping_mul(2);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old_ptr = if a.is_null() {
        null_mut()
    } else {
        header_from_array(a) as *mut c_void
    };
    let size = elemsize
        .wrapping_mul(min_cap)
        .wrapping_add(size_of::<StbdsArrayHeader>());
    let b = realloc(old_ptr, size);
    if b.is_null() {
        return null_mut();
    }

    let array = (b as *mut u8).add(size_of::<StbdsArrayHeader>()) as *mut c_void;
    let h = header_from_array(array);
    if a.is_null() {
        (*h).length = 0;
        (*h).hash_table = null_mut();
        (*h).temp = 0;
    }
    (*h).capacity = min_cap;
    array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    if !a.is_null() {
        free(header_from_array(a) as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    STBDS_HASH_SEED = seed;
}

fn rotate_left(value: usize, n: u32) -> usize {
    value.rotate_left(n)
}

fn rotate_right(value: usize, n: u32) -> usize {
    value.rotate_right(n)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(mut str_: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    while *str_ != 0 {
        hash = rotate_left(hash, 9).wrapping_add(*str_ as u8 as usize);
        str_ = str_.add(1);
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

fn sip_round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotate_left(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotate_left(*v0, (usize::BITS / 2) as u32);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotate_left(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotate_left(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotate_left(*v2, (usize::BITS / 2) as u32);
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotate_left(*v3, 21);
    *v3 ^= *v0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut v0 = (((0x736f_6d65usize << 16) << 16).wrapping_add(0x7073_6575)) ^ seed;
    let mut v1 = (((0x646f_7261usize << 16) << 16).wrapping_add(0x6e64_6f6d)) ^ !seed;
    let mut v2 = (((0x6c79_6765usize << 16) << 16).wrapping_add(0x6e65_7261)) ^ seed;
    let mut v3 = (((0x7465_6462usize << 16) << 16).wrapping_add(0x7974_6573)) ^ !seed;

    v0 ^= 0x0706_0504_0302_0100usize ^ seed;
    v1 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
    v2 ^= 0x0706_0504_0302_0100usize ^ seed;
    v3 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

    let mut i = 0usize;
    while i + size_of::<usize>() <= len {
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
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
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
        1 => {
            data |= *d.add(0) as usize;
        }
        _ => {}
    }

    v3 ^= data;
    for _ in 0..2 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut c_void, _elemsize: usize) {
    if a.is_null() {
        return;
    }
    let raw = header_from_array(a);
    if !(*raw).hash_table.is_null() {
        free((*raw).hash_table);
    }
    free(raw as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(
    a: *mut c_void,
    elemsize: usize,
    _key: *mut c_void,
    _keysize: usize,
    temp: *mut isize,
    _mode: c_int,
) -> *mut c_void {
    if a.is_null() {
        let arr = stbds_arrgrowf(null_mut(), elemsize, 0, 1);
        if !arr.is_null() {
            (*header_from_array(arr)).length = (*header_from_array(arr)).length.wrapping_add(1);
            memset(arr, 0, elemsize);
        }
        if !temp.is_null() {
            *temp = -1;
        }
        return arr_to_hash(arr, elemsize);
    }
    if !temp.is_null() {
        *temp = -1;
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
    let mut temp = 0isize;
    let p = stbds_hmget_key_ts(a, elemsize, key, keysize, &mut temp, mode);
    if !p.is_null() {
        let raw = hash_to_arr(p, elemsize);
        (*header_from_array(raw)).temp = temp;
    }
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    if a.is_null() {
        let arr = stbds_arrgrowf(null_mut(), elemsize, 0, 1);
        if !arr.is_null() {
            (*header_from_array(arr)).length = (*header_from_array(arr)).length.wrapping_add(1);
            memset(arr, 0, elemsize);
        }
        return arr_to_hash(arr, elemsize);
    }
    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    _mode: c_int,
) -> *mut c_void {
    let hash_array = if a.is_null() {
        stbds_hmput_default(a, elemsize)
    } else {
        a
    };
    if hash_array.is_null() {
        return hash_array;
    }

    let raw = hash_to_arr(hash_array, elemsize);
    let index = (*header_from_array(raw)).length;
    let grown = stbds_arrgrowf(raw, elemsize, 1, 0);
    if grown.is_null() {
        return null_mut();
    }
    let item = (grown as *mut u8).add(index.wrapping_mul(elemsize)) as *mut c_void;
    memmove(item, key, keysize);
    (*header_from_array(grown)).length = index.wrapping_add(1);
    (*header_from_array(grown)).temp = index as isize - 1;
    arr_to_hash(grown, elemsize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(
    a: *mut c_void,
    _elemsize: usize,
    _key: *mut c_void,
    _keysize: usize,
    _keyoffset: usize,
    _mode: c_int,
) -> *mut c_void {
    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, _mode: c_int) -> *mut c_void {
    let arr = stbds_arrgrowf(null_mut(), elemsize, 0, 1);
    if !arr.is_null() {
        memset(arr, 0, elemsize);
        (*header_from_array(arr)).length = 1;
    }
    arr_to_hash(arr, elemsize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut StbdsStringArena,
    str_: *mut c_char,
) -> *mut c_char {
    const BLOCKSIZE_MIN: usize = 512;
    const BLOCKSIZE_MAX: usize = 1 << 20;

    let len = strlen(str_).wrapping_add(1);
    if len > (*a).remaining {
        let blocksize = BLOCKSIZE_MIN << ((*a).block >> 1);
        if blocksize < BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let alloc_size = size_of::<StbdsStringBlock>().wrapping_sub(8).wrapping_add(len);
            let sb = realloc(null_mut(), alloc_size) as *mut StbdsStringBlock;
            if sb.is_null() {
                return null_mut();
            }
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
        }

        let alloc_size = size_of::<StbdsStringBlock>()
            .wrapping_sub(8)
            .wrapping_add(blocksize);
        let sb = realloc(null_mut(), alloc_size) as *mut StbdsStringBlock;
        if sb.is_null() {
            return null_mut();
        }
        (*sb).next = (*a).storage;
        (*a).storage = sb;
        (*a).remaining = blocksize;
    }

    let p = ((*(*a).storage).storage.as_mut_ptr() as *mut u8)
        .add((*a).remaining.wrapping_sub(len)) as *mut c_char;
    (*a).remaining = (*a).remaining.wrapping_sub(len);
    memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut StbdsStringArena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        free(x as *mut c_void);
        x = y;
    }
    memset(
        a as *mut c_void,
        0,
        size_of::<StbdsStringArena>(),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *mut c_char {
    let buf = addr_of_mut!(STRKEY_BUFFER) as *mut c_char;
    snprintf(buf, 256, c"test_%d".as_ptr(), n);
    buf
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn str_dups(num: c_int) {
    let mut sa = StbdsStringArena {
        storage: null_mut(),
        remaining: 0,
        block: 0,
        mode: 0,
    };

    let mut i = 0;
    while i < num {
        let key = strkey(i);
        stbds_stralloc(&mut sa, key);
        i += 1;
    }
    stbds_strreset(&mut sa);

    printf(c"a %d\n".as_ptr(), num);
}
