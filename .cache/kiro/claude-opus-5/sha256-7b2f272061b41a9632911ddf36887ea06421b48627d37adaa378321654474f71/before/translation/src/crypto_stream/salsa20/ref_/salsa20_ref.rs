//! Translation of c_src/libsodium/crypto_stream/salsa20/ref/salsa20_ref.c

// HAVE_AMD64_ASM undefined: the whole `#ifndef HAVE_AMD64_ASM` body is compiled.

use core::ffi::{c_int, c_uint, c_void};

// #[repr(C)] copy of `crypto_stream_salsa20_implementation` from
// crypto_stream/salsa20/stream_salsa20.h.
#[repr(C)]
pub struct CryptoStreamSalsa20Implementation {
    pub stream: Option<
        unsafe extern "C" fn(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int,
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

unsafe impl Sync for CryptoStreamSalsa20Implementation {}

extern "C" {
    fn crypto_core_salsa20(
        out: *mut u8,
        in_: *const u8,
        k: *const u8,
        c: *const u8,
    ) -> c_int;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

unsafe extern "C" fn stream_ref(
    c: *mut u8,
    mut clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut in_: [u8; 16] = [0u8; 16];
    let mut block: [u8; 64] = [0u8; 64];
    let mut kcopy: [u8; 32] = [0u8; 32];
    let mut i: c_uint;
    let mut u: c_uint;
    let mut c = c;

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
        crypto_core_salsa20(c, in_.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        u = 1;
        i = 8;
        while i < 16 {
            u += in_[i as usize] as c_uint;
            in_[i as usize] = u as u8;
            u >>= 8;
            i += 1;
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
        i = 0;
        while (i as u64) < clen {
            *c.add(i as usize) = block[i as usize];
            i += 1;
        }
    }
    sodium_memzero(block.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&block));
    sodium_memzero(kcopy.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&kcopy));

    0
}

unsafe extern "C" fn stream_ref_xor_ic(
    c: *mut u8,
    m: *const u8,
    mut mlen: u64,
    n: *const u8,
    mut ic: u64,
    k: *const u8,
) -> c_int {
    let mut in_: [u8; 16] = [0u8; 16];
    let mut block: [u8; 64] = [0u8; 64];
    let mut kcopy: [u8; 32] = [0u8; 32];
    let mut i: c_uint;
    let mut u: c_uint;
    let mut c = c;
    let mut m = m;

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
        crypto_core_salsa20(
            block.as_mut_ptr(),
            in_.as_ptr(),
            kcopy.as_ptr(),
            core::ptr::null(),
        );
        i = 0;
        while i < 64 {
            *c.add(i as usize) = *m.add(i as usize) ^ block[i as usize];
            i += 1;
        }
        u = 1;
        i = 8;
        while i < 16 {
            u += in_[i as usize] as c_uint;
            in_[i as usize] = u as u8;
            u >>= 8;
            i += 1;
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
        i = 0;
        while (i as u64) < mlen {
            *c.add(i as usize) = *m.add(i as usize) ^ block[i as usize];
            i += 1;
        }
    }
    sodium_memzero(block.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&block));
    sodium_memzero(kcopy.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&kcopy));

    0
}

// Non-static C variable: exported symbol.
#[unsafe(no_mangle)]
pub static crypto_stream_salsa20_ref_implementation: CryptoStreamSalsa20Implementation =
    CryptoStreamSalsa20Implementation {
        stream: Some(stream_ref),
        stream_xor_ic: Some(stream_ref_xor_ic),
    };
