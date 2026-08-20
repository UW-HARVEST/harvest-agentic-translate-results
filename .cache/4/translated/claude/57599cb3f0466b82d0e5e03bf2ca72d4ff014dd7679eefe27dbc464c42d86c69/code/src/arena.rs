//! String storage: `stbds_strdup`, `stbds_stralloc`, `stbds_strreset`.

use core::ffi::c_char;

use crate::ffi::*;

/// ```c
/// static char *stbds_strdup(char *str)
/// ```
pub unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = strlen(str_) + 1;
    let p = realloc(core::ptr::null_mut(), len) as *mut c_char;
    memmove(p as *mut _, str_ as *const _, len);
    p
}

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

/// ```c
/// char *stbds_stralloc(stbds_string_arena *a, char *str)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn stbds_stralloc(a: *mut StbdsStringArena, str_: *mut c_char) -> *mut c_char {
    unsafe {
        let p: *mut c_char;
        let len = strlen(str_) + 1;
        if len > (*a).remaining {
            let mut blocksize: usize = (*a).block as usize;

            blocksize = STBDS_STRING_ARENA_BLOCKSIZE_MIN << (blocksize >> 1);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                (*a).block = (*a).block.wrapping_add(1);
            }

            if len > blocksize {
                let sb = realloc(
                    core::ptr::null_mut(),
                    core::mem::size_of::<StbdsStringBlock>() - 8 + len,
                ) as *mut StbdsStringBlock;
                memmove(
                    core::ptr::addr_of_mut!((*sb).storage) as *mut _,
                    str_ as *const _,
                    len,
                );
                if !(*a).storage.is_null() {
                    (*sb).next = (*(*a).storage).next;
                    (*(*a).storage).next = sb;
                } else {
                    (*sb).next = core::ptr::null_mut();
                    (*a).storage = sb;
                    (*a).remaining = 0;
                }
                return core::ptr::addr_of_mut!((*sb).storage) as *mut c_char;
            } else {
                let sb = realloc(
                    core::ptr::null_mut(),
                    core::mem::size_of::<StbdsStringBlock>() - 8 + blocksize,
                ) as *mut StbdsStringBlock;
                (*sb).next = (*a).storage;
                (*a).storage = sb;
                (*a).remaining = blocksize;
            }
        }

        stbds_assert!(len <= (*a).remaining);
        p = (core::ptr::addr_of_mut!((*(*a).storage).storage) as *mut c_char)
            .wrapping_add((*a).remaining)
            .wrapping_sub(len);
        (*a).remaining -= len;
        memmove(p as *mut _, str_ as *const _, len);
        p
    }
}

/// ```c
/// void stbds_strreset(stbds_string_arena *a)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn stbds_strreset(a: *mut StbdsStringArena) {
    unsafe {
        let mut x: *mut StbdsStringBlock;
        let mut y: *mut StbdsStringBlock;
        x = (*a).storage;
        while !x.is_null() {
            y = (*x).next;
            free(x as *mut _);
            x = y;
        }
        core::ptr::write_bytes(a as *mut u8, 0, core::mem::size_of::<StbdsStringArena>());
    }
}
