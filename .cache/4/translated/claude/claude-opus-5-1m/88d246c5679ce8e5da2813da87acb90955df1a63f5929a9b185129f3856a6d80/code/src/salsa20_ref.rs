//! Translation of `crypto_stream/salsa20/ref/salsa20_ref.c`
//!
//! version 20140420, D. J. Bernstein, Public domain.
//!
//! The reference build does not define `HAVE_AMD64_ASM`, so the whole file is
//! compiled.

use crate::common::*;
use core::ffi::{c_int, c_uint, c_ulonglong, c_void};

extern "C" {
    fn crypto_core_salsa20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

/// `typedef struct crypto_stream_salsa20_implementation` from
/// `crypto_stream/salsa20/stream_salsa20.h`.
#[repr(C)]
pub struct crypto_stream_salsa20_implementation {
    pub stream: unsafe extern "C" fn(
        c: *mut u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int,
    pub stream_xor_ic: unsafe extern "C" fn(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        ic: u64,
        k: *const u8,
    ) -> c_int,
}

unsafe extern "C" fn stream_ref(
    mut c: *mut u8,
    mut clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut in_: [u8; 16] = [0; 16];
    let mut block: [u8; 64] = [0; 64];
    let mut kcopy: [u8; 32] = [0; 32];
    let mut u: c_uint;

    if clen == 0 {
        return 0;
    }
    for i in 0..32usize {
        kcopy[i] = *k.add(i);
    }
    for i in 0..8usize {
        in_[i] = *n.add(i);
    }
    for i in 8..16usize {
        in_[i] = 0;
    }
    while clen >= 64 {
        crypto_core_salsa20(c, in_.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        u = 1;
        for i in 8..16usize {
            u = u.wrapping_add(in_[i] as c_uint);
            in_[i] = u as u8;
            u >>= 8;
        }
        clen -= 64;
        c = c.add(64);
    }
    if clen != 0 {
        crypto_core_salsa20(
            block.as_mut_ptr(),
            in_.as_ptr(),
            kcopy.as_ptr(),
            core::ptr::null(),
        );
        let n_tail = clen as c_uint;
        let mut i: c_uint = 0;
        while i < n_tail {
            *c.add(i as usize) = block[i as usize];
            i = i.wrapping_add(1);
        }
    }
    sodium_memzero(block.as_mut_ptr() as *mut c_void, 64);
    sodium_memzero(kcopy.as_mut_ptr() as *mut c_void, 32);

    0
}

unsafe extern "C" fn stream_ref_xor_ic(
    mut c: *mut u8,
    mut m: *const u8,
    mut mlen: c_ulonglong,
    n: *const u8,
    mut ic: u64,
    k: *const u8,
) -> c_int {
    let mut in_: [u8; 16] = [0; 16];
    let mut block: [u8; 64] = [0; 64];
    let mut kcopy: [u8; 32] = [0; 32];
    let mut u: c_uint;

    if mlen == 0 {
        return 0;
    }
    for i in 0..32usize {
        kcopy[i] = *k.add(i);
    }
    for i in 0..8usize {
        in_[i] = *n.add(i);
    }
    for i in 8..16usize {
        in_[i] = (ic & 0xff) as u8;
        ic >>= 8;
    }
    while mlen >= 64 {
        crypto_core_salsa20(
            block.as_mut_ptr(),
            in_.as_ptr(),
            kcopy.as_ptr(),
            core::ptr::null(),
        );
        for i in 0..64usize {
            *c.add(i) = *m.add(i) ^ block[i];
        }
        u = 1;
        for i in 8..16usize {
            u = u.wrapping_add(in_[i] as c_uint);
            in_[i] = u as u8;
            u >>= 8;
        }
        mlen -= 64;
        c = c.add(64);
        m = m.add(64);
    }
    if mlen != 0 {
        crypto_core_salsa20(
            block.as_mut_ptr(),
            in_.as_ptr(),
            kcopy.as_ptr(),
            core::ptr::null(),
        );
        let n_tail = mlen as c_uint;
        let mut i: c_uint = 0;
        while i < n_tail {
            *c.add(i as usize) = *m.add(i as usize) ^ block[i as usize];
            i = i.wrapping_add(1);
        }
    }
    sodium_memzero(block.as_mut_ptr() as *mut c_void, 64);
    sodium_memzero(kcopy.as_mut_ptr() as *mut c_void, 32);

    0
}

#[unsafe(no_mangle)]
pub static crypto_stream_salsa20_ref_implementation: crypto_stream_salsa20_implementation =
    crypto_stream_salsa20_implementation {
        stream: stream_ref,
        stream_xor_ic: stream_ref_xor_ic,
    };
