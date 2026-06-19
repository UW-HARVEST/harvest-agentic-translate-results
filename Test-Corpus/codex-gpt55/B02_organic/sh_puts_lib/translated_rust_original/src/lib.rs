use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

#[repr(C)]
struct StbdsArrayHeader {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
pub struct StbdsStringBlock {
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

static mut STBDS_HASH_SEED: usize = 0x31415926;
static mut STRKEY_BUFFER: [c_char; 256] = [0; 256];

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(dst: *mut c_void, value: c_int, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
}

unsafe fn header(a: *mut c_void) -> *mut StbdsArrayHeader {
    unsafe { (a as *mut StbdsArrayHeader).sub(1) }
}

unsafe fn arr_len(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*header(a)).length }
    }
}

unsafe fn arr_cap(a: *mut c_void) -> usize {
    if a.is_null() {
        0
    } else {
        unsafe { (*header(a)).capacity }
    }
}

unsafe fn arr_to_hash(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (a as *mut u8).add(elemsize) as *mut c_void }
}

unsafe fn hash_to_arr(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe { (a as *mut u8).sub(elemsize) as *mut c_void }
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_rand_seed(seed: usize) {
    unsafe {
        STBDS_HASH_SEED = seed;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hash_string(str_: *mut c_char, seed: usize) -> usize {
    let mut hash = seed;
    let mut p = str_ as *const u8;
    unsafe {
        while *p != 0 {
            hash = hash.rotate_left(9).wrapping_add(*p as usize);
            p = p.add(1);
        }
    }

    hash ^= seed;
    hash = (!hash).wrapping_add(hash.wrapping_shl(18));
    hash ^= hash ^ hash.rotate_right(31);
    hash = hash.wrapping_mul(21);
    hash ^= hash ^ hash.rotate_right(11);
    hash = hash.wrapping_add(hash.wrapping_shl(6));
    hash ^= hash.rotate_right(22);
    hash.wrapping_add(seed)
}

fn sip_round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = v1.rotate_left(13);
    *v1 ^= *v0;
    *v0 = v0.rotate_left(usize::BITS / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = v3.rotate_left(16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = v1.rotate_left(17);
    *v1 ^= *v2;
    *v2 = v2.rotate_left(usize::BITS / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = v3.rotate_left(21);
    *v3 ^= *v0;
}

fn siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut v0 = (((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed;
    let mut v1 = (((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    let mut v2 = (((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
    let mut v3 = (((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let mut i = 0usize;
    unsafe {
        while i + size_of::<usize>() <= len {
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
            1 => data |= *d.add(0) as usize,
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
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    siphash_bytes(p, len, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut c_void {
    unsafe {
        let min_len = arr_len(a).wrapping_add(addlen);
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

        let old = if a.is_null() {
            ptr::null_mut()
        } else {
            header(a) as *mut c_void
        };
        let size = elemsize
            .wrapping_mul(min_cap)
            .wrapping_add(size_of::<StbdsArrayHeader>());
        let b = realloc(old, size);
        let b_data = (b as *mut u8).add(size_of::<StbdsArrayHeader>()) as *mut c_void;
        if a.is_null() {
            (*header(b_data)).length = 0;
            (*header(b_data)).hash_table = ptr::null_mut();
            (*header(b_data)).temp = 0;
        }
        (*header(b_data)).capacity = min_cap;
        b_data
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_arrfreef(a: *mut c_void) {
    unsafe {
        free(header(a) as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmfree_func(a: *mut c_void, _elemsize: usize) {
    if a.is_null() {
        return;
    }
    unsafe {
        let h = header(a);
        if !(*h).hash_table.is_null() {
            free((*h).hash_table);
        }
        free(h as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmget_key_ts(
    a: *mut c_void,
    elemsize: usize,
    _key: *mut c_void,
    _keysize: usize,
    temp: *mut isize,
    _mode: c_int,
) -> *mut c_void {
    unsafe {
        if a.is_null() {
            let p = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            (*header(p)).length += 1;
            memset(p, 0, elemsize);
            *temp = -1;
            arr_to_hash(p, elemsize)
        } else {
            *temp = -1;
            a
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmget_key(
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
pub extern "C" fn stbds_hmput_default(a: *mut c_void, elemsize: usize) -> *mut c_void {
    unsafe {
        if a.is_null() || (*header(hash_to_arr(a, elemsize))).length == 0 {
            let raw = stbds_arrgrowf(
                if a.is_null() {
                    ptr::null_mut()
                } else {
                    hash_to_arr(a, elemsize)
                },
                elemsize,
                0,
                1,
            );
            (*header(raw)).length += 1;
            memset(raw, 0, elemsize);
            arr_to_hash(raw, elemsize)
        } else {
            a
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmput_key(
    a: *mut c_void,
    elemsize: usize,
    key: *mut c_void,
    keysize: usize,
    mode: c_int,
) -> *mut c_void {
    unsafe {
        let mut a = if a.is_null() {
            let raw = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
            memset(raw, 0, elemsize);
            (*header(raw)).length += 1;
            arr_to_hash(raw, elemsize)
        } else {
            a
        };
        let raw = hash_to_arr(a, elemsize);
        let i = arr_len(raw);
        let raw = if i + 1 > arr_cap(raw) {
            stbds_arrgrowf(raw, elemsize, 1, 0)
        } else {
            raw
        };
        (*header(raw)).length = i + 1;
        let slot = (raw as *mut u8).add(elemsize * i);
        if mode >= 1 {
            *(slot as *mut *mut c_char) = key as *mut c_char;
        } else {
            memmove(slot as *mut c_void, key, keysize);
        }
        (*header(raw)).temp = (i as isize) - 1;
        a = arr_to_hash(raw, elemsize);
        a
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut c_void {
    unsafe {
        let a = stbds_arrgrowf(ptr::null_mut(), elemsize, 0, 1);
        memset(a, 0, elemsize);
        (*header(a)).length = 1;
        (*header(a)).hash_table = realloc(ptr::null_mut(), 1);
        if !(*header(a)).hash_table.is_null() {
            *(*header(a)).hash_table.cast::<u8>() = mode as u8;
        }
        arr_to_hash(a, elemsize)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hmdel_key(
    a: *mut c_void,
    elemsize: usize,
    _key: *mut c_void,
    _keysize: usize,
    _keyoffset: usize,
    _mode: c_int,
) -> *mut c_void {
    unsafe {
        if !a.is_null() {
            (*header(hash_to_arr(a, elemsize))).temp = 0;
        }
    }
    a
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_stralloc(a: *mut StbdsStringArena, str_: *mut c_char) -> *mut c_char {
    unsafe {
        let len = strlen(str_) + 1;
        if len > (*a).remaining {
            let blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (((*a).block as usize) >> 1);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }

            if len > blocksize {
                let sb = realloc(
                    ptr::null_mut(),
                    size_of::<StbdsStringBlock>() - 8 + len,
                ) as *mut StbdsStringBlock;
                let storage = ptr::addr_of_mut!((*sb).storage) as *mut c_char;
                memmove(storage as *mut c_void, str_ as *const c_void, len);
                if !(*a).storage.is_null() {
                    (*sb).next = (*(*a).storage).next;
                    (*(*a).storage).next = sb;
                } else {
                    (*sb).next = ptr::null_mut();
                    (*a).storage = sb;
                    (*a).remaining = 0;
                }
                return storage;
            } else {
                let sb = realloc(
                    ptr::null_mut(),
                    size_of::<StbdsStringBlock>() - 8 + blocksize,
                ) as *mut StbdsStringBlock;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        let storage = ptr::addr_of_mut!((*(*a).storage).storage) as *mut c_char;
        let p = storage.add((*a).remaining - len);
        (*a).remaining -= len;
        memmove(p as *mut c_void, str_ as *const c_void, len);
        p
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_strreset(a: *mut StbdsStringArena) {
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
            size_of::<StbdsStringArena>(),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe {
        let buf = ptr::addr_of_mut!(STRKEY_BUFFER) as *mut c_char;
        sprintf(buf, c"test_%d".as_ptr(), n);
        buf
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sh_puts(num: c_int) {
    unsafe {
        printf(c"%s %d\n".as_ptr(), c"a".as_ptr(), num);
    }
}
