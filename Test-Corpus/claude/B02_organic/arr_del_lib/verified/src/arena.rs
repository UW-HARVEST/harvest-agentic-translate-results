//! String duplication and the string arena.

use core::ffi::{c_char, c_void};
use core::mem::size_of;
use core::ptr::null_mut;

use crate::*;

/// ```c
/// static char *stbds_strdup(char *str)
/// ```
pub(crate) unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = strlen(str_).wrapping_add(1);
    let p = STBDS_REALLOC(null_mut(), len) as *mut c_char;
    memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

/// ```c
/// char *stbds_stralloc(stbds_string_arena *a, char *str)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    let p: *mut c_char;
    let len = strlen(str_).wrapping_add(1);
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;

        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN.wrapping_shl((blocksize >> 1) as u32);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb = STBDS_REALLOC(
                null_mut(),
                size_of::<stbds_string_block>().wrapping_sub(8).wrapping_add(len),
            ) as *mut stbds_string_block;
            memmove(
                (&raw mut (*sb).storage) as *mut c_void,
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
            return (&raw mut (*sb).storage) as *mut c_char;
        } else {
            let sb = STBDS_REALLOC(
                null_mut(),
                size_of::<stbds_string_block>()
                    .wrapping_sub(8)
                    .wrapping_add(blocksize),
            ) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    STBDS_ASSERT(len <= (*a).remaining);
    p = ((&raw mut (*(*a).storage).storage) as *mut c_char)
        .wrapping_add((*a).remaining)
        .wrapping_sub(len);
    (*a).remaining = (*a).remaining.wrapping_sub(len);
    memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

/// ```c
/// void stbds_strreset(stbds_string_arena *a)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x: *mut stbds_string_block;
    let mut y: *mut stbds_string_block;
    x = (*a).storage;
    while !x.is_null() {
        y = (*x).next;
        STBDS_FREE(x as *mut c_void);
        x = y;
    }
    memset(a as *mut c_void, 0, size_of::<stbds_string_arena>());
}
