//! String duplication and the string arena: `stbds_strdup`, `stbds_stralloc`,
//! `stbds_strreset`.

use core::ffi::{c_char, c_void};
use core::mem::size_of;
use core::ptr;

use crate::c;
use crate::types::*;

/// `static char *stbds_strdup(char *str)`
pub unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = c::strlen(str_) + 1;
    let p = c::realloc(ptr::null_mut(), len) as *mut c_char;
    c::memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

/// `char *stbds_stralloc(stbds_string_arena *a, char *str)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut StringArena, str_: *mut c_char) -> *mut c_char {
    let len = c::strlen(str_) + 1;
    if len > (*a).remaining {
        let mut blocksize = (*a).block as usize;

        blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            // `sizeof(*sb) - 8 + len`
            let sb = c::realloc(
                ptr::null_mut(),
                (size_of::<StringBlock>() - 8).wrapping_add(len),
            ) as *mut StringBlock;
            c::memmove(
                (&raw mut (*sb).storage) as *mut c_void,
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
            return (&raw mut (*sb).storage) as *mut c_char;
        } else {
            let sb = c::realloc(
                ptr::null_mut(),
                (size_of::<StringBlock>() - 8).wrapping_add(blocksize),
            ) as *mut StringBlock;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    stbds_assert!(
        len <= (*a).remaining,
        "len <= a->remaining",
        913,
        "stbds_stralloc"
    );
    let p = ((&raw mut (*(*a).storage).storage) as *mut c_char)
        .wrapping_add((*a).remaining.wrapping_sub(len) as usize);
    (*a).remaining -= len;
    c::memmove(p as *mut c_void, str_ as *const c_void, len);
    p
}

/// `void stbds_strreset(stbds_string_arena *a)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut StringArena) {
    let mut x = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        c::free(x as *mut c_void);
        x = y;
    }
    c::memset(a as *mut c_void, 0, size_of::<StringArena>());
}
