//! Translation of `crypto_stream/salsa208/stream_salsa208.c` and
//! `crypto_stream/salsa208/ref/stream_salsa208_ref.c`.
//!
//! ```text
//! version 20140420
//! D. J. Bernstein
//! Public domain.
//! ```

use core::ffi::{c_int, c_void};

use crate::common::SODIUM_SIZE_MAX;
use crate::randombytes::randombytes_buf;
use crate::sodium::utils::sodium_memzero;

unsafe extern "C" {
    fn crypto_core_salsa208(out: *mut u8, inp: *const u8, k: *const u8, c: *const u8) -> c_int;
}

pub const crypto_stream_salsa208_KEYBYTES: usize = 32;
pub const crypto_stream_salsa208_NONCEBYTES: usize = 8;

// ---------------------------------------------------------------------------
// stream_salsa208.c
// ---------------------------------------------------------------------------

/* LCOV_EXCL_START */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208_keybytes() -> usize {
    crypto_stream_salsa208_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208_noncebytes() -> usize {
    crypto_stream_salsa208_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208_messagebytes_max() -> usize {
    SODIUM_SIZE_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_salsa208_KEYBYTES);
}

// ---------------------------------------------------------------------------
// ref/stream_salsa208_ref.c
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208(
    mut c: *mut u8,
    mut clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut in_: [u8; 16] = [0; 16];
    let mut block: [u8; 64] = [0; 64];
    let mut kcopy: [u8; 32] = [0; 32];
    let mut i: u32;
    let mut u: u32;

    if clen == 0 {
        return 0;
    }
    i = 0;
    while i < 32 {
        kcopy[i as usize] = unsafe { *k.add(i as usize) };
        i += 1;
    }
    i = 0;
    while i < 8 {
        in_[i as usize] = unsafe { *n.add(i as usize) };
        i += 1;
    }
    i = 8;
    while i < 16 {
        in_[i as usize] = 0;
        i += 1;
    }
    while clen >= 64 {
        unsafe {
            crypto_core_salsa208(c, in_.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        }
        u = 1;
        i = 8;
        while i < 16 {
            u = u.wrapping_add(in_[i as usize] as u32);
            in_[i as usize] = u as u8;
            u >>= 8;
            i += 1;
        }
        clen -= 64;
        c = unsafe { c.add(64) };
    }
    if clen != 0 {
        unsafe {
            crypto_core_salsa208(
                block.as_mut_ptr(),
                in_.as_ptr(),
                kcopy.as_ptr(),
                core::ptr::null(),
            );
        }
        i = 0;
        while i < clen as u32 {
            unsafe {
                *c.add(i as usize) = block[i as usize];
            }
            i += 1;
        }
    }
    unsafe {
        sodium_memzero(block.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&block));
        sodium_memzero(kcopy.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&kcopy));
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208_xor(
    mut c: *mut u8,
    mut m: *const u8,
    mut mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut in_: [u8; 16] = [0; 16];
    let mut block: [u8; 64] = [0; 64];
    let mut kcopy: [u8; 32] = [0; 32];
    let mut i: u32;
    let mut u: u32;

    if mlen == 0 {
        return 0;
    }
    i = 0;
    while i < 32 {
        kcopy[i as usize] = unsafe { *k.add(i as usize) };
        i += 1;
    }
    i = 0;
    while i < 8 {
        in_[i as usize] = unsafe { *n.add(i as usize) };
        i += 1;
    }
    i = 8;
    while i < 16 {
        in_[i as usize] = 0;
        i += 1;
    }
    while mlen >= 64 {
        unsafe {
            crypto_core_salsa208(
                block.as_mut_ptr(),
                in_.as_ptr(),
                kcopy.as_ptr(),
                core::ptr::null(),
            );
        }
        i = 0;
        while i < 64 {
            unsafe {
                *c.add(i as usize) = *m.add(i as usize) ^ block[i as usize];
            }
            i += 1;
        }
        u = 1;
        i = 8;
        while i < 16 {
            u = u.wrapping_add(in_[i as usize] as u32);
            in_[i as usize] = u as u8;
            u >>= 8;
            i += 1;
        }
        mlen -= 64;
        c = unsafe { c.add(64) };
        m = unsafe { m.add(64) };
    }
    if mlen != 0 {
        unsafe {
            crypto_core_salsa208(
                block.as_mut_ptr(),
                in_.as_ptr(),
                kcopy.as_ptr(),
                core::ptr::null(),
            );
        }
        i = 0;
        while i < mlen as u32 {
            unsafe {
                *c.add(i as usize) = *m.add(i as usize) ^ block[i as usize];
            }
            i += 1;
        }
    }
    unsafe {
        sodium_memzero(block.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&block));
        sodium_memzero(kcopy.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&kcopy));
    }

    0
}

/* LCOV_EXCL_STOP */
