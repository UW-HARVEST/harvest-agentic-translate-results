//! Translated from salsa2012 and salsa208 stream ref implementations.
use crate::primitives::cutil::*;
use core::ffi::c_void;

extern "C" {
    fn crypto_core_salsa2012(out: *mut u8, inp: *const u8, k: *const u8, c: *const u8) -> i32;
    fn crypto_core_salsa208(out: *mut u8, inp: *const u8, k: *const u8, c: *const u8) -> i32;
}

#[inline(always)]
fn messagebytes_max() -> u64 {
    core::cmp::min(u64::MAX, usize::MAX as u64)
}

// ------- salsa2012 -------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa2012(
    c: *mut u8,
    mut clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    let mut inbuf = [0u8; 16];
    let mut block = [0u8; 64];
    let mut kcopy = [0u8; 32];
    if clen == 0 {
        return 0;
    }
    for i in 0..32 {
        kcopy[i] = *k.add(i);
    }
    for i in 0..8 {
        inbuf[i] = *n.add(i);
    }
    for i in 8..16 {
        inbuf[i] = 0;
    }
    let mut c = c;
    while clen >= 64 {
        crypto_core_salsa2012(c, inbuf.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        let mut u: u32 = 1;
        for i in 8..16 {
            u += inbuf[i] as u32;
            inbuf[i] = u as u8;
            u >>= 8;
        }
        clen -= 64;
        c = c.add(64);
    }
    if clen != 0 {
        crypto_core_salsa2012(block.as_mut_ptr(), inbuf.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        for i in 0..(clen as usize) {
            *c.add(i) = block[i];
        }
    }
    sodium_memzero(block.as_mut_ptr() as *mut c_void, 64);
    sodium_memzero(kcopy.as_mut_ptr() as *mut c_void, 32);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa2012_xor(
    c: *mut u8,
    m: *const u8,
    mut mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    let mut inbuf = [0u8; 16];
    let mut block = [0u8; 64];
    let mut kcopy = [0u8; 32];
    if mlen == 0 {
        return 0;
    }
    for i in 0..32 {
        kcopy[i] = *k.add(i);
    }
    for i in 0..8 {
        inbuf[i] = *n.add(i);
    }
    for i in 8..16 {
        inbuf[i] = 0;
    }
    let mut c = c;
    let mut m = m;
    while mlen >= 64 {
        crypto_core_salsa2012(block.as_mut_ptr(), inbuf.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        for i in 0..64 {
            *c.add(i) = *m.add(i) ^ block[i];
        }
        let mut u: u32 = 1;
        for i in 8..16 {
            u += inbuf[i] as u32;
            inbuf[i] = u as u8;
            u >>= 8;
        }
        mlen -= 64;
        c = c.add(64);
        m = m.add(64);
    }
    if mlen != 0 {
        crypto_core_salsa2012(block.as_mut_ptr(), inbuf.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        for i in 0..(mlen as usize) {
            *c.add(i) = *m.add(i) ^ block[i];
        }
    }
    sodium_memzero(block.as_mut_ptr() as *mut c_void, 64);
    sodium_memzero(kcopy.as_mut_ptr() as *mut c_void, 32);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_salsa2012_keybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_salsa2012_noncebytes() -> usize {
    8
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_salsa2012_messagebytes_max() -> usize {
    messagebytes_max() as usize
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa2012_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}

// ------- salsa208 -------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208(
    c: *mut u8,
    mut clen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    let mut inbuf = [0u8; 16];
    let mut block = [0u8; 64];
    let mut kcopy = [0u8; 32];
    if clen == 0 {
        return 0;
    }
    for i in 0..32 {
        kcopy[i] = *k.add(i);
    }
    for i in 0..8 {
        inbuf[i] = *n.add(i);
    }
    for i in 8..16 {
        inbuf[i] = 0;
    }
    let mut c = c;
    while clen >= 64 {
        crypto_core_salsa208(c, inbuf.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        let mut u: u32 = 1;
        for i in 8..16 {
            u += inbuf[i] as u32;
            inbuf[i] = u as u8;
            u >>= 8;
        }
        clen -= 64;
        c = c.add(64);
    }
    if clen != 0 {
        crypto_core_salsa208(block.as_mut_ptr(), inbuf.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        for i in 0..(clen as usize) {
            *c.add(i) = block[i];
        }
    }
    sodium_memzero(block.as_mut_ptr() as *mut c_void, 64);
    sodium_memzero(kcopy.as_mut_ptr() as *mut c_void, 32);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208_xor(
    c: *mut u8,
    m: *const u8,
    mut mlen: u64,
    n: *const u8,
    k: *const u8,
) -> i32 {
    let mut inbuf = [0u8; 16];
    let mut block = [0u8; 64];
    let mut kcopy = [0u8; 32];
    if mlen == 0 {
        return 0;
    }
    for i in 0..32 {
        kcopy[i] = *k.add(i);
    }
    for i in 0..8 {
        inbuf[i] = *n.add(i);
    }
    for i in 8..16 {
        inbuf[i] = 0;
    }
    let mut c = c;
    let mut m = m;
    while mlen >= 64 {
        crypto_core_salsa208(block.as_mut_ptr(), inbuf.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        for i in 0..64 {
            *c.add(i) = *m.add(i) ^ block[i];
        }
        let mut u: u32 = 1;
        for i in 8..16 {
            u += inbuf[i] as u32;
            inbuf[i] = u as u8;
            u >>= 8;
        }
        mlen -= 64;
        c = c.add(64);
        m = m.add(64);
    }
    if mlen != 0 {
        crypto_core_salsa208(block.as_mut_ptr(), inbuf.as_ptr(), kcopy.as_ptr(), core::ptr::null());
        for i in 0..(mlen as usize) {
            *c.add(i) = *m.add(i) ^ block[i];
        }
    }
    sodium_memzero(block.as_mut_ptr() as *mut c_void, 64);
    sodium_memzero(kcopy.as_mut_ptr() as *mut c_void, 32);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_salsa208_keybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_salsa208_noncebytes() -> usize {
    8
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_stream_salsa208_messagebytes_max() -> usize {
    messagebytes_max() as usize
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 32);
}
