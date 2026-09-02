//! String arena support: `stbds_stralloc`, `stbds_strreset` and the internal
//! `stbds_strdup`.

use core::ffi::{c_char, c_void};
use core::ptr;

use crate::{
    free, realloc, stbds_assert, stbds_string_arena, stbds_string_block, strlen,
};

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: u32 = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: u32 = 1 << 20;

/// ```c
/// static char *stbds_strdup(char *str)
/// ```
pub(crate) unsafe fn stbds_strdup(str_: *mut c_char) -> *mut c_char {
    let len = strlen(str_) + 1;
    let p = realloc(ptr::null_mut(), len) as *mut c_char;
    ptr::copy(str_ as *const u8, p as *mut u8, len);
    p
}

/// ```c
/// char *stbds_stralloc(stbds_string_arena *a, char *str)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(
    a: *mut stbds_string_arena,
    str_: *mut c_char,
) -> *mut c_char {
    let p: *mut c_char;
    let len = strlen(str_) + 1;
    if len > (*a).remaining {
        let mut blocksize: usize = (*a).block as usize;

        // ```c
        // blocksize = (size_t) (STBDS_STRING_ARENA_BLOCKSIZE_MIN) << (blocksize>>1);
        // ```
        //
        // `a->block` saturates at 22 when the arena is only ever driven through
        // this function, so `blocksize >> 1 <= 11`. A caller handing over an
        // arena with a larger `block` (it is a plain `unsigned char` field) makes
        // the shift count exceed the width of `size_t`, which is UB in C. On
        // x86-64 gcc that compiles to a `shl` whose count register is taken
        // modulo 64, so mask explicitly to reproduce the reference behaviour
        // instead of leaving it to the Rust backend.
        blocksize = (STBDS_STRING_ARENA_BLOCKSIZE_MIN as usize)
            << ((blocksize >> 1) & (usize::BITS as usize - 1));

        if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX as usize {
            (*a).block = (*a).block.wrapping_add(1);
        }

        if len > blocksize {
            let sb = realloc(
                ptr::null_mut(),
                core::mem::size_of::<stbds_string_block>() - 8 + len,
            ) as *mut stbds_string_block;
            ptr::copy(
                str_ as *const u8,
                ptr::addr_of_mut!((*sb).storage) as *mut u8,
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
                core::mem::size_of::<stbds_string_block>() - 8 + blocksize,
            ) as *mut stbds_string_block;
            (*sb).next = (*a).storage;
            (*a).storage = sb;
            (*a).remaining = blocksize;
        }
    }

    stbds_assert!(len <= (*a).remaining);
    p = (ptr::addr_of_mut!((*(*a).storage).storage) as *mut c_char)
        .wrapping_add((*a).remaining)
        .wrapping_sub(len);
    (*a).remaining -= len;
    ptr::copy(str_ as *const u8, p as *mut u8, len);
    p
}

/// ```c
/// void stbds_strreset(stbds_string_arena *a)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds_string_arena) {
    let mut x: *mut stbds_string_block = (*a).storage;
    while !x.is_null() {
        let y = (*x).next;
        free(x as *mut c_void);
        x = y;
    }
    ptr::write_bytes(
        a as *mut u8,
        0,
        core::mem::size_of::<stbds_string_arena>(),
    );
}
