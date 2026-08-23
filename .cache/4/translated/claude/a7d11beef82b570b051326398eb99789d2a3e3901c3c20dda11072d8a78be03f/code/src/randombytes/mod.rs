//! `randombytes/randombytes.c`

pub mod internal;
pub mod os;
pub mod sysrandom;

use core::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn crypto_stream_chacha20_ietf(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
}

pub const RANDOMBYTES_SEEDBYTES: usize = 32;

/// `randombytes_BYTES_MAX` == min(SODIUM_SIZE_MAX, 0xffffffff)
pub const RANDOMBYTES_BYTES_MAX: u64 = 0xffffffff;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RandombytesImplementation {
    pub implementation_name: Option<extern "C" fn() -> *const c_char>,
    pub random: Option<extern "C" fn() -> u32>,
    pub stir: Option<extern "C" fn()>,
    pub uniform: Option<extern "C" fn(u32) -> u32>,
    pub buf: Option<unsafe extern "C" fn(*mut c_void, usize)>,
    pub close: Option<extern "C" fn() -> c_int>,
}

unsafe impl Sync for RandombytesImplementation {}

static mut IMPLEMENTATION: *const RandombytesImplementation = core::ptr::null();

fn randombytes_init_if_needed() {
    if unsafe { IMPLEMENTATION }.is_null() {
        unsafe {
            IMPLEMENTATION = &raw const sysrandom::randombytes_sysrandom_implementation;
        }
        randombytes_stir();
    }
}

#[inline]
fn impl_ref() -> &'static RandombytesImplementation {
    unsafe { &*IMPLEMENTATION }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_set_implementation(
    r#impl: *const RandombytesImplementation,
) -> c_int {
    unsafe { IMPLEMENTATION = r#impl };
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_implementation_name() -> *const c_char {
    randombytes_init_if_needed();
    (impl_ref().implementation_name.unwrap())()
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_random() -> u32 {
    randombytes_init_if_needed();
    (impl_ref().random.unwrap())()
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_stir() {
    randombytes_init_if_needed();
    if let Some(stir) = impl_ref().stir {
        stir();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_uniform(upper_bound: u32) -> u32 {
    randombytes_init_if_needed();
    if let Some(uniform) = impl_ref().uniform {
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
pub extern "C" fn randombytes_buf(buf: *mut c_void, size: usize) {
    randombytes_init_if_needed();
    if size > 0 {
        unsafe { (impl_ref().buf.unwrap())(buf, size) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_buf_deterministic(
    buf: *mut c_void,
    size: usize,
    seed: *const u8,
) {
    static NONCE: [u8; 12] = [b'L', b'i', b'b', b's', b'o', b'd', b'i', b'u', b'm', b'D', b'R', b'G'];

    // #if SIZE_MAX > 0x4000000000ULL  (true on 64-bit)
    if (size as u64) > 0x4000000000u64 {
        crate::sodium::core::sodium_misuse();
    }
    unsafe {
        crypto_stream_chacha20_ietf(buf as *mut u8, size as u64, NONCE.as_ptr(), seed)
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_seedbytes() -> usize {
    RANDOMBYTES_SEEDBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_close() -> c_int {
    if !unsafe { IMPLEMENTATION }.is_null() {
        if let Some(close) = impl_ref().close {
            return close();
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(buf: *mut u8, buf_len: u64) {
    randombytes_buf(buf as *mut c_void, buf_len as usize);
}
