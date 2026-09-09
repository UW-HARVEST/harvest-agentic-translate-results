//! Rust translation of `randombytes/randombytes.c`.

use core::ffi::{c_char, c_int, c_void};

use crate::types::randombytes_implementation;

extern "C" {
    fn sodium_misuse() -> !;
    fn crypto_stream_chacha20_ietf(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
    #[link_name = "randombytes_sysrandom_implementation"]
    static RANDOMBYTES_SYSRANDOM_IMPLEMENTATION: randombytes_implementation;
}

static mut IMPLEMENTATION: *const randombytes_implementation = core::ptr::null();

unsafe fn randombytes_init_if_needed() {
    if IMPLEMENTATION.is_null() {
        IMPLEMENTATION = &RANDOMBYTES_SYSRANDOM_IMPLEMENTATION as *const randombytes_implementation;
        randombytes_stir();
    }
}

#[no_mangle]
pub unsafe extern "C" fn randombytes_set_implementation(
    impl_: *const randombytes_implementation,
) -> c_int {
    IMPLEMENTATION = impl_;
    0
}

#[no_mangle]
pub unsafe extern "C" fn randombytes_implementation_name() -> *const c_char {
    randombytes_init_if_needed();
    ((*IMPLEMENTATION).implementation_name.unwrap_unchecked())()
}

#[no_mangle]
pub unsafe extern "C" fn randombytes_random() -> u32 {
    randombytes_init_if_needed();
    ((*IMPLEMENTATION).random.unwrap_unchecked())()
}

#[no_mangle]
pub unsafe extern "C" fn randombytes_stir() {
    randombytes_init_if_needed();
    if let Some(stir) = (*IMPLEMENTATION).stir {
        stir();
    }
}

#[no_mangle]
pub unsafe extern "C" fn randombytes_uniform(upper_bound: u32) -> u32 {
    randombytes_init_if_needed();
    if let Some(uniform) = (*IMPLEMENTATION).uniform {
        return uniform(upper_bound);
    }
    if upper_bound < 2 {
        return 0;
    }
    let min: u32 = (1u32.wrapping_add(!upper_bound)) % upper_bound;
    let mut r: u32;
    loop {
        r = randombytes_random();
        if r >= min {
            break;
        }
    }

    r % upper_bound
}

#[no_mangle]
pub unsafe extern "C" fn randombytes_buf(buf: *mut c_void, size: usize) {
    randombytes_init_if_needed();
    if size > 0 {
        ((*IMPLEMENTATION).buf.unwrap_unchecked())(buf, size);
    }
}

#[no_mangle]
pub unsafe extern "C" fn randombytes_buf_deterministic(
    buf: *mut c_void,
    size: usize,
    seed: *const u8,
) {
    static NONCE: [u8; 12] = [
        b'L', b'i', b'b', b's', b'o', b'd', b'i', b'u', b'm', b'D', b'R', b'G',
    ];

    if size > 0x4000000000u64 as usize {
        sodium_misuse();
    }

    crypto_stream_chacha20_ietf(buf as *mut u8, size as u64, NONCE.as_ptr(), seed);
}

#[no_mangle]
pub unsafe extern "C" fn randombytes_seedbytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn randombytes_close() -> c_int {
    if !IMPLEMENTATION.is_null() {
        if let Some(close) = (*IMPLEMENTATION).close {
            return close();
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn randombytes(buf: *mut u8, buf_len: u64) {
    // assert(buf_len <= SIZE_MAX); -- always true: both are 64-bit on this target.
    randombytes_buf(buf as *mut c_void, buf_len as usize);
}
