//! Translation of `libsodium/randombytes/randombytes.c`

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use super::{randombytes_implementation, RANDOMBYTES_SEEDBYTES};

static mut IMPLEMENTATION: *const randombytes_implementation = ptr::null();

extern "C" {
    fn crypto_stream_chacha20_ietf(
        c: *mut u8,
        clen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
}

unsafe fn randombytes_init_if_needed() {
    if IMPLEMENTATION.is_null() {
        IMPLEMENTATION = ptr::addr_of!(super::sysrandom::randombytes_sysrandom_implementation);
        randombytes_stir();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_set_implementation(
    impl_: *const randombytes_implementation,
) -> c_int {
    IMPLEMENTATION = impl_;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_implementation_name() -> *const c_char {
    randombytes_init_if_needed();
    ((*IMPLEMENTATION).implementation_name.unwrap())()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_random() -> u32 {
    randombytes_init_if_needed();
    ((*IMPLEMENTATION).random.unwrap())()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_stir() {
    randombytes_init_if_needed();
    if let Some(stir) = (*IMPLEMENTATION).stir {
        stir();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_uniform(upper_bound: u32) -> u32 {
    randombytes_init_if_needed();
    if let Some(uniform) = (*IMPLEMENTATION).uniform {
        return uniform(upper_bound);
    }
    if upper_bound < 2 {
        return 0;
    }
    let min = (1u32.wrapping_add(!upper_bound)) % upper_bound;
    let mut r;
    loop {
        r = randombytes_random();
        if r >= min {
            break;
        }
    }

    r % upper_bound
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_buf(buf: *mut c_void, size: usize) {
    randombytes_init_if_needed();
    if size > 0 {
        ((*IMPLEMENTATION).buf.unwrap())(buf, size);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_buf_deterministic(
    buf: *mut c_void,
    size: usize,
    seed: *const u8,
) {
    static NONCE: [u8; 12] = [b'L', b'i', b'b', b's', b'o', b'd', b'i', b'u', b'm', b'D', b'R', b'G'];

    if size > 0x4000000000u64 as usize {
        crate::sodium::core::sodium_misuse();
    }
    crypto_stream_chacha20_ietf(buf as *mut u8, size as u64, NONCE.as_ptr(), seed);
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_seedbytes() -> usize {
    RANDOMBYTES_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_close() -> c_int {
    if !IMPLEMENTATION.is_null() {
        if let Some(close) = (*IMPLEMENTATION).close {
            return close();
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(buf: *mut u8, buf_len: u64) {
    randombytes_buf(buf as *mut c_void, buf_len as usize);
}
