//! String arena, translated from `c_src/src/lib.c`.

use core::ffi::{c_char, c_void};
use core::ptr;

use crate::cffi::{free, memmove, memset, realloc, strlen};
use crate::types::*;

pub const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
pub const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

/// ```c
/// static char *stbds_strdup(char *str)
/// { size_t len = strlen(str)+1; char *p = (char*) STBDS_REALLOC(NULL, 0, len); memmove(p, str, len); return p; }
/// ```
pub unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = strlen(str_) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

/// ```c
/// char *stbds_stralloc(stbds_string_arena *a, char *str)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut StbdsStringArena,
    str_: *mut c_char,
) -> *mut c_char {
    let p: *mut c_char;
    let len: usize = strlen(str_) + 1;
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;

        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb: *mut StbdsStringBlock = realloc(
                ptr::null_mut(),
                core::mem::size_of::<StbdsStringBlock>()
                    .wrapping_sub(8)
                    .wrapping_add(len),
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
            let sb: *mut StbdsStringBlock = realloc(
                ptr::null_mut(),
                core::mem::size_of::<StbdsStringBlock>()
                    .wrapping_sub(8)
                    .wrapping_add(blocksize),
            ) as *mut StbdsStringBlock;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    stbds_assert!(
        len <= (*a).remaining,
        "len <= a->remaining\0",
        913,
        "stbds_stralloc\0"
    );
    p = (ptr::addr_of_mut!((*(*a).storage).storage) as *mut c_char)
        .wrapping_add((*a).remaining)
        .wrapping_sub(len);
    (*a).remaining -= len;
    memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

/// ```c
/// void stbds_strreset(stbds_string_arena *a)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut StbdsStringArena) {
    let mut x: *mut StbdsStringBlock;
    let mut y: *mut StbdsStringBlock;
    x = (*a).storage;
    while !x.is_null() {
        y = (*x).next;
        free(x as *mut c_void);
        x = y;
    }
    memset(
        a as *mut c_void,
        0,
        core::mem::size_of::<StbdsStringArena>(),
    );
}
