//! Translation of crypto_stream/salsa20/stream_salsa20.c,
//! crypto_stream/salsa20/stream_salsa20.h and
//! crypto_stream/salsa20/ref/salsa20_ref.c.

use core::ffi::{c_int, c_void};
use core::ptr;

use crate::common::SODIUM_SIZE_MAX;
use crate::crypto_core::salsa::crypto_core_salsa20;
use crate::randombytes::randombytes_buf;
use crate::sodium_utils::sodium_memzero;

pub const crypto_stream_salsa20_KEYBYTES: usize = 32;
pub const crypto_stream_salsa20_NONCEBYTES: usize = 8;
pub const crypto_stream_salsa20_MESSAGEBYTES_MAX: usize = SODIUM_SIZE_MAX;

/* ------------------------------------------------------------------ */
/* stream_salsa20.h                                                    */
/* ------------------------------------------------------------------ */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_stream_salsa20_implementation {
    pub stream: Option<
        unsafe extern "C" fn(
            c: *mut u8,
            clen: u64,
            n: *const u8,
            k: *const u8,
        ) -> c_int,
    >,
    pub stream_xor_ic: Option<
        unsafe extern "C" fn(
            c: *mut u8,
            m: *const u8,
            mlen: u64,
            n: *const u8,
            ic: u64,
            k: *const u8,
        ) -> c_int,
    >,
}

unsafe impl Sync for crypto_stream_salsa20_implementation {}

/* ------------------------------------------------------------------ */
/* ref/salsa20_ref.c                                                   */
/* ------------------------------------------------------------------ */

unsafe extern "C" fn stream_ref(
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
        kcopy[i as usize] = *k.add(i as usize);
        i += 1;
    }
    i = 0;
    while i < 8 {
        in_[i as usize] = *n.add(i as usize);
        i += 1;
    }
    i = 8;
    while i < 16 {
        in_[i as usize] = 0;
        i += 1;
    }
    while clen >= 64 {
        crypto_core_salsa20(c, in_.as_ptr(), kcopy.as_ptr(), ptr::null());
        u = 1;
        i = 8;
        while i < 16 {
            u = u.wrapping_add(in_[i as usize] as u32);
            in_[i as usize] = u as u8;
            u >>= 8;
            i += 1;
        }
        clen -= 64;
        c = c.add(64);
    }
    if clen != 0 {
        crypto_core_salsa20(block.as_mut_ptr(), in_.as_ptr(), kcopy.as_ptr(), ptr::null());
        i = 0;
        while i < clen as u32 {
            *c.add(i as usize) = block[i as usize];
            i += 1;
        }
    }
    sodium_memzero(block.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&block));
    sodium_memzero(kcopy.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&kcopy));

    0
}

unsafe extern "C" fn stream_ref_xor_ic(
    mut c: *mut u8,
    mut m: *const u8,
    mut mlen: u64,
    n: *const u8,
    mut ic: u64,
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
        kcopy[i as usize] = *k.add(i as usize);
        i += 1;
    }
    i = 0;
    while i < 8 {
        in_[i as usize] = *n.add(i as usize);
        i += 1;
    }
    i = 8;
    while i < 16 {
        in_[i as usize] = (ic & 0xff) as u8;
        ic >>= 8;
        i += 1;
    }
    while mlen >= 64 {
        crypto_core_salsa20(block.as_mut_ptr(), in_.as_ptr(), kcopy.as_ptr(), ptr::null());
        i = 0;
        while i < 64 {
            *c.add(i as usize) = *m.add(i as usize) ^ block[i as usize];
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
        c = c.add(64);
        m = m.add(64);
    }
    if mlen != 0 {
        crypto_core_salsa20(block.as_mut_ptr(), in_.as_ptr(), kcopy.as_ptr(), ptr::null());
        i = 0;
        while i < mlen as u32 {
            *c.add(i as usize) = *m.add(i as usize) ^ block[i as usize];
            i += 1;
        }
    }
    sodium_memzero(block.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&block));
    sodium_memzero(kcopy.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&kcopy));

    0
}

#[unsafe(no_mangle)]
pub static crypto_stream_salsa20_ref_implementation: crypto_stream_salsa20_implementation =
    crypto_stream_salsa20_implementation {
        stream: Some(stream_ref),
        stream_xor_ic: Some(stream_ref_xor_ic),
    };

/* ------------------------------------------------------------------ */
/* stream_salsa20.c                                                    */
/* ------------------------------------------------------------------ */

static mut implementation: *const crypto_stream_salsa20_implementation =
    &crypto_stream_salsa20_ref_implementation;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_salsa20_keybytes() -> usize {
    crypto_stream_salsa20_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_salsa20_noncebytes() -> usize {
    crypto_stream_salsa20_NONCEBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_salsa20_messagebytes_max() -> usize {
    crypto_stream_salsa20_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    ((*implementation).stream.unwrap())(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    ((*implementation).stream_xor_ic.unwrap())(c, m, mlen, n, ic, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    ((*implementation).stream_xor_ic.unwrap())(c, m, mlen, n, 0u64, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa20_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_salsa20_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_stream_salsa20_pick_best_implementation() -> c_int {
    implementation = &crypto_stream_salsa20_ref_implementation;
    0 /* LCOV_EXCL_LINE */
}
