//! Translation of:
//!   - `crypto_stream/salsa20/stream_salsa20.c` + `crypto_stream/salsa20/ref/salsa20_ref.c`
//!   - `crypto_stream/salsa2012/stream_salsa2012.c` + `crypto_stream/salsa2012/ref/stream_salsa2012_ref.c`
//!   - `crypto_stream/salsa208/stream_salsa208.c` + `crypto_stream/salsa208/ref/stream_salsa208_ref.c`
//!   - `crypto_stream/xsalsa20/stream_xsalsa20.c`
//!
//! The reference build has no SIMD implementations, so `pick_best_implementation`
//! always selects the `ref` implementation.

use crate::common::SODIUM_SIZE_MAX;
use core::ffi::c_int;

extern "C" {
    fn crypto_core_salsa20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;
    fn crypto_core_salsa2012(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;
    fn crypto_core_salsa208(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;
    fn crypto_core_hsalsa20(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8) -> c_int;
    fn sodium_memzero(pnt: *mut u8, len: usize);
    fn randombytes_buf(buf: *mut u8, size: usize);
}

// =====================================================================
// crypto_stream/salsa20/stream_salsa20.h
// =====================================================================

#[repr(C)]
pub struct crypto_stream_salsa20_implementation {
    pub stream: unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int,
    pub stream_xor_ic:
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> c_int,
}

// =====================================================================
// crypto_stream/salsa20/ref/salsa20_ref.c
// =====================================================================

unsafe extern "C" fn stream_ref(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int {
    let mut in_ = [0u8; 16];
    let mut block = [0u8; 64];
    let mut kcopy = [0u8; 32];
    let mut u: u32;
    let mut c = c;
    let mut clen = clen;

    if clen == 0 {
        return 0;
    }
    for i in 0..32 {
        kcopy[i] = *k.add(i);
    }
    for i in 0..8 {
        in_[i] = *n.add(i);
    }
    for i in 8..16 {
        in_[i] = 0;
    }
    while clen >= 64 {
        crypto_core_salsa20(c, in_.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        u = 1;
        for i in 8..16 {
            u = u.wrapping_add(in_[i] as u32);
            in_[i] = u as u8;
            u >>= 8;
        }
        clen -= 64;
        c = c.add(64);
    }
    if clen != 0 {
        crypto_core_salsa20(block.as_mut_ptr(), in_.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        for i in 0..(clen as usize) {
            *c.add(i) = block[i];
        }
    }
    sodium_memzero(block.as_mut_ptr(), block.len());
    sodium_memzero(kcopy.as_mut_ptr(), kcopy.len());

    0
}

unsafe extern "C" fn stream_ref_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    let mut in_ = [0u8; 16];
    let mut block = [0u8; 64];
    let mut kcopy = [0u8; 32];
    let mut u: u32;
    let mut c = c;
    let mut m = m;
    let mut mlen = mlen;
    let mut ic = ic;

    if mlen == 0 {
        return 0;
    }
    for i in 0..32 {
        kcopy[i] = *k.add(i);
    }
    for i in 0..8 {
        in_[i] = *n.add(i);
    }
    for i in 8..16 {
        in_[i] = (ic & 0xff) as u8;
        ic >>= 8;
    }
    while mlen >= 64 {
        crypto_core_salsa20(block.as_mut_ptr(), in_.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        for i in 0..64 {
            *c.add(i) = *m.add(i) ^ block[i];
        }
        u = 1;
        for i in 8..16 {
            u = u.wrapping_add(in_[i] as u32);
            in_[i] = u as u8;
            u >>= 8;
        }
        mlen -= 64;
        c = c.add(64);
        m = m.add(64);
    }
    if mlen != 0 {
        crypto_core_salsa20(block.as_mut_ptr(), in_.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        for i in 0..(mlen as usize) {
            *c.add(i) = *m.add(i) ^ block[i];
        }
    }
    sodium_memzero(block.as_mut_ptr(), block.len());
    sodium_memzero(kcopy.as_mut_ptr(), kcopy.len());

    0
}

#[no_mangle]
pub static crypto_stream_salsa20_ref_implementation: crypto_stream_salsa20_implementation =
    crypto_stream_salsa20_implementation {
        stream: stream_ref,
        stream_xor_ic: stream_ref_xor_ic,
    };

// =====================================================================
// crypto_stream/salsa20/stream_salsa20.c
// =====================================================================

static mut IMPLEMENTATION: *const crypto_stream_salsa20_implementation =
    &crypto_stream_salsa20_ref_implementation;

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa20_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa20_noncebytes() -> usize {
    8
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa20_messagebytes_max() -> usize {
    SODIUM_SIZE_MAX as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa20(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    ((*IMPLEMENTATION).stream)(c, clen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    ((*IMPLEMENTATION).stream_xor_ic)(c, m, mlen, n, ic, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    ((*IMPLEMENTATION).stream_xor_ic)(c, m, mlen, n, 0, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa20_keygen(k: *mut u8) {
    randombytes_buf(k, 32);
}

#[no_mangle]
pub unsafe extern "C" fn _crypto_stream_salsa20_pick_best_implementation() -> c_int {
    IMPLEMENTATION = &crypto_stream_salsa20_ref_implementation;
    0
}

// =====================================================================
// crypto_stream/salsa2012/ref/stream_salsa2012_ref.c
// (no SIMD/implementation indirection exists for salsa2012)
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa2012(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut in_ = [0u8; 16];
    let mut block = [0u8; 64];
    let mut kcopy = [0u8; 32];
    let mut u: u32;
    let mut c = c;
    let mut clen = clen;

    if clen == 0 {
        return 0;
    }
    for i in 0..32 {
        kcopy[i] = *k.add(i);
    }
    for i in 0..8 {
        in_[i] = *n.add(i);
    }
    for i in 8..16 {
        in_[i] = 0;
    }
    while clen >= 64 {
        crypto_core_salsa2012(c, in_.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        u = 1;
        for i in 8..16 {
            u = u.wrapping_add(in_[i] as u32);
            in_[i] = u as u8;
            u >>= 8;
        }
        clen -= 64;
        c = c.add(64);
    }
    if clen != 0 {
        crypto_core_salsa2012(block.as_mut_ptr(), in_.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        for i in 0..(clen as usize) {
            *c.add(i) = block[i];
        }
    }
    sodium_memzero(block.as_mut_ptr(), block.len());
    sodium_memzero(kcopy.as_mut_ptr(), kcopy.len());

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa2012_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut in_ = [0u8; 16];
    let mut block = [0u8; 64];
    let mut kcopy = [0u8; 32];
    let mut u: u32;
    let mut c = c;
    let mut m = m;
    let mut mlen = mlen;

    if mlen == 0 {
        return 0;
    }
    for i in 0..32 {
        kcopy[i] = *k.add(i);
    }
    for i in 0..8 {
        in_[i] = *n.add(i);
    }
    for i in 8..16 {
        in_[i] = 0;
    }
    while mlen >= 64 {
        crypto_core_salsa2012(block.as_mut_ptr(), in_.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        for i in 0..64 {
            *c.add(i) = *m.add(i) ^ block[i];
        }
        u = 1;
        for i in 8..16 {
            u = u.wrapping_add(in_[i] as u32);
            in_[i] = u as u8;
            u >>= 8;
        }
        mlen -= 64;
        c = c.add(64);
        m = m.add(64);
    }
    if mlen != 0 {
        crypto_core_salsa2012(block.as_mut_ptr(), in_.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        for i in 0..(mlen as usize) {
            *c.add(i) = *m.add(i) ^ block[i];
        }
    }
    sodium_memzero(block.as_mut_ptr(), block.len());
    sodium_memzero(kcopy.as_mut_ptr(), kcopy.len());

    0
}

// =====================================================================
// crypto_stream/salsa2012/stream_salsa2012.c
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa2012_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa2012_noncebytes() -> usize {
    8
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa2012_messagebytes_max() -> usize {
    SODIUM_SIZE_MAX as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa2012_keygen(k: *mut u8) {
    randombytes_buf(k, 32);
}

// =====================================================================
// crypto_stream/salsa208/ref/stream_salsa208_ref.c
// (no SIMD/implementation indirection exists for salsa208)
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa208(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut in_ = [0u8; 16];
    let mut block = [0u8; 64];
    let mut kcopy = [0u8; 32];
    let mut u: u32;
    let mut c = c;
    let mut clen = clen;

    if clen == 0 {
        return 0;
    }
    for i in 0..32 {
        kcopy[i] = *k.add(i);
    }
    for i in 0..8 {
        in_[i] = *n.add(i);
    }
    for i in 8..16 {
        in_[i] = 0;
    }
    while clen >= 64 {
        crypto_core_salsa208(c, in_.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        u = 1;
        for i in 8..16 {
            u = u.wrapping_add(in_[i] as u32);
            in_[i] = u as u8;
            u >>= 8;
        }
        clen -= 64;
        c = c.add(64);
    }
    if clen != 0 {
        crypto_core_salsa208(block.as_mut_ptr(), in_.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        for i in 0..(clen as usize) {
            *c.add(i) = block[i];
        }
    }
    sodium_memzero(block.as_mut_ptr(), block.len());
    sodium_memzero(kcopy.as_mut_ptr(), kcopy.len());

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa208_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut in_ = [0u8; 16];
    let mut block = [0u8; 64];
    let mut kcopy = [0u8; 32];
    let mut u: u32;
    let mut c = c;
    let mut m = m;
    let mut mlen = mlen;

    if mlen == 0 {
        return 0;
    }
    for i in 0..32 {
        kcopy[i] = *k.add(i);
    }
    for i in 0..8 {
        in_[i] = *n.add(i);
    }
    for i in 8..16 {
        in_[i] = 0;
    }
    while mlen >= 64 {
        crypto_core_salsa208(block.as_mut_ptr(), in_.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        for i in 0..64 {
            *c.add(i) = *m.add(i) ^ block[i];
        }
        u = 1;
        for i in 8..16 {
            u = u.wrapping_add(in_[i] as u32);
            in_[i] = u as u8;
            u >>= 8;
        }
        mlen -= 64;
        c = c.add(64);
        m = m.add(64);
    }
    if mlen != 0 {
        crypto_core_salsa208(block.as_mut_ptr(), in_.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        for i in 0..(mlen as usize) {
            *c.add(i) = *m.add(i) ^ block[i];
        }
    }
    sodium_memzero(block.as_mut_ptr(), block.len());
    sodium_memzero(kcopy.as_mut_ptr(), kcopy.len());

    0
}

// =====================================================================
// crypto_stream/salsa208/stream_salsa208.c
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa208_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa208_noncebytes() -> usize {
    8
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa208_messagebytes_max() -> usize {
    SODIUM_SIZE_MAX as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_salsa208_keygen(k: *mut u8) {
    randombytes_buf(k, 32);
}

// =====================================================================
// crypto_stream/xsalsa20/stream_xsalsa20.c
// =====================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_xsalsa20(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut subkey = [0u8; 32];

    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());
    let ret = crypto_stream_salsa20(c, clen, n.add(16), subkey.as_ptr());
    sodium_memzero(subkey.as_mut_ptr(), subkey.len());

    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_xsalsa20_xor_ic(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    ic: u64,
    k: *const u8,
) -> c_int {
    let mut subkey = [0u8; 32];

    crypto_core_hsalsa20(subkey.as_mut_ptr(), n, k, core::ptr::null());
    let ret = crypto_stream_salsa20_xor_ic(c, m, mlen, n.add(16), ic, subkey.as_ptr());
    sodium_memzero(subkey.as_mut_ptr(), subkey.len());

    ret
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_xsalsa20_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_stream_xsalsa20_xor_ic(c, m, mlen, n, 0, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_xsalsa20_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_xsalsa20_noncebytes() -> usize {
    24
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_xsalsa20_messagebytes_max() -> usize {
    SODIUM_SIZE_MAX as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_xsalsa20_keygen(k: *mut u8) {
    randombytes_buf(k, 32);
}
