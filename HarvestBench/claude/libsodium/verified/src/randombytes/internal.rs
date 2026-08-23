//! `randombytes/internal/randombytes_internal_random.c`
//!
//! `TLS` expands to nothing (the build uses `-std=gnu99`, so
//! `__STDC_VERSION__ < 201112L`), `HAVE_GETPID` and `HAVE_RDRAND` are not
//! defined, and Linux gets `HAVE_LINUX_COMPATIBLE_GETRANDOM` via the
//! `syscall(SYS_getrandom, ...)` shim.

use core::ffi::{c_char, c_int, c_void};

use super::RandombytesImplementation;
use super::os;
use crate::common::{get_errno, set_errno};
use crate::sodium::core::sodium_misuse;
use crate::sodium::utils::sodium_memzero;

unsafe extern "C" {
    fn crypto_stream_chacha20(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
    fn crypto_stream_chacha20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
}

const CRYPTO_STREAM_CHACHA20_KEYBYTES: usize = 32;
/// `crypto_core_hchacha20_OUTPUTBYTES`
const INTERNAL_RANDOM_BLOCK_SIZE: usize = 32;

#[repr(C)]
struct InternalRandomGlobal {
    initialized: c_int,
    random_data_source_fd: c_int,
    getentropy_available: c_int,
    getrandom_available: c_int,
    rdrand_available: c_int,
}

#[repr(C)]
struct InternalRandom {
    initialized: c_int,
    rnd32_outleft: usize,
    key: [u8; CRYPTO_STREAM_CHACHA20_KEYBYTES],
    rnd32: [u8; 16 * INTERNAL_RANDOM_BLOCK_SIZE],
    nonce: u64,
}

static mut GLOBAL: InternalRandomGlobal = InternalRandomGlobal {
    initialized: 0,
    random_data_source_fd: -1,
    getentropy_available: 0,
    getrandom_available: 0,
    rdrand_available: 0,
};

static mut STREAM: InternalRandom = InternalRandom {
    initialized: 0,
    rnd32_outleft: 0,
    key: [0; CRYPTO_STREAM_CHACHA20_KEYBYTES],
    rnd32: [0; 16 * INTERNAL_RANDOM_BLOCK_SIZE],
    nonce: 0,
};

#[inline]
fn g() -> &'static mut InternalRandomGlobal {
    unsafe { &mut *(&raw mut GLOBAL) }
}

#[inline]
fn st() -> &'static mut InternalRandom {
    unsafe { &mut *(&raw mut STREAM) }
}

fn randombytes_internal_random_init() {
    let errno_save = get_errno();

    g().rdrand_available = crate::sodium::runtime::sodium_runtime_has_rdrand();
    g().getentropy_available = 0;
    g().getrandom_available = 0;

    {
        let mut fodder = [0u8; 16];
        if unsafe { os::linux_getrandom(fodder.as_mut_ptr() as *mut c_void, 16) } == 0 {
            g().getrandom_available = 1;
            set_errno(errno_save);
            return;
        }
    }

    g().random_data_source_fd = os::random_dev_open();
    if g().random_data_source_fd == -1 {
        sodium_misuse();
    }
    set_errno(errno_save);
}

extern "C" fn randombytes_internal_random_stir() {
    st().nonce = os::sodium_hrtime();
    // assert(stream.nonce != (uint64_t) 0U) -- live; fires only if
    // gettimeofday() yields exactly 0 microseconds since the epoch.
    if st().nonce == 0 {
        unsafe { crate::common::abort() };
    }
    st().rnd32 = [0u8; 16 * INTERNAL_RANDOM_BLOCK_SIZE];
    st().rnd32_outleft = 0;
    if g().initialized == 0 {
        randombytes_internal_random_init();
        g().initialized = 1;
    }

    if g().getrandom_available != 0 {
        if unsafe {
            os::linux_getrandom(
                st().key.as_mut_ptr() as *mut c_void,
                CRYPTO_STREAM_CHACHA20_KEYBYTES,
            )
        } != 0
        {
            sodium_misuse();
        }
    }

    st().initialized = 1;
}

fn randombytes_internal_random_stir_if_needed() {
    if st().initialized == 0 {
        randombytes_internal_random_stir();
    }
}

extern "C" fn randombytes_internal_random_close() -> c_int {
    let mut ret: c_int = -1;

    if g().getrandom_available != 0 {
        ret = 0;
    }

    unsafe {
        sodium_memzero(
            (&raw mut STREAM) as *mut c_void,
            core::mem::size_of::<InternalRandom>(),
        )
    };

    ret
}

/// `randombytes_internal_random_xorhwrand()` -- `HAVE_RDRAND` is undefined so
/// this is a no-op.
fn randombytes_internal_random_xorhwrand() {}

fn randombytes_internal_random_xorkey(mix: &[u8; CRYPTO_STREAM_CHACHA20_KEYBYTES]) {
    let s = st();
    for i in 0..CRYPTO_STREAM_CHACHA20_KEYBYTES {
        s.key[i] ^= mix[i];
    }
}

unsafe extern "C" fn randombytes_internal_random_buf(buf: *mut c_void, size: usize) {
    randombytes_internal_random_stir_if_needed();
    let s = st();
    let ret = unsafe {
        crypto_stream_chacha20(
            buf as *mut u8,
            size as u64,
            (&raw mut s.nonce) as *const u8,
            s.key.as_ptr(),
        )
    };
    // assert(ret == 0) -- live (no NDEBUG); unreachable in practice.
    if ret != 0 {
        unsafe { crate::common::abort() };
    }
    let size_bytes = size.to_ne_bytes();
    for i in 0..core::mem::size_of::<usize>() {
        s.key[i] ^= size_bytes[i];
    }
    randombytes_internal_random_xorhwrand();
    s.nonce = s.nonce.wrapping_add(1);
    unsafe {
        crypto_stream_chacha20_xor(
            s.key.as_mut_ptr(),
            s.key.as_ptr(),
            CRYPTO_STREAM_CHACHA20_KEYBYTES as u64,
            (&raw mut s.nonce) as *const u8,
            s.key.as_ptr(),
        )
    };
}

extern "C" fn randombytes_internal_random() -> u32 {
    let s = st();
    if s.rnd32_outleft == 0 {
        randombytes_internal_random_stir_if_needed();
        let ret = unsafe {
            crypto_stream_chacha20(
                s.rnd32.as_mut_ptr(),
                (16 * INTERNAL_RANDOM_BLOCK_SIZE) as u64,
                (&raw mut s.nonce) as *const u8,
                s.key.as_ptr(),
            )
        };
        // assert(ret == 0) -- live (no NDEBUG); unreachable in practice.
        if ret != 0 {
            unsafe { crate::common::abort() };
        }
        s.rnd32_outleft = (16 * INTERNAL_RANDOM_BLOCK_SIZE) - CRYPTO_STREAM_CHACHA20_KEYBYTES;
        randombytes_internal_random_xorhwrand();
        let mut mix = [0u8; CRYPTO_STREAM_CHACHA20_KEYBYTES];
        mix.copy_from_slice(&s.rnd32[s.rnd32_outleft..s.rnd32_outleft + 32]);
        randombytes_internal_random_xorkey(&mix);
        let off = s.rnd32_outleft;
        s.rnd32[off..off + CRYPTO_STREAM_CHACHA20_KEYBYTES].fill(0);
        s.nonce = s.nonce.wrapping_add(1);
    }
    s.rnd32_outleft -= 4;
    let off = s.rnd32_outleft;
    let val = u32::from_ne_bytes([
        s.rnd32[off],
        s.rnd32[off + 1],
        s.rnd32[off + 2],
        s.rnd32[off + 3],
    ]);
    s.rnd32[off..off + 4].fill(0);

    val
}

extern "C" fn randombytes_internal_implementation_name() -> *const c_char {
    b"internal\0".as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub static mut randombytes_internal_implementation: RandombytesImplementation =
    RandombytesImplementation {
        implementation_name: Some(randombytes_internal_implementation_name),
        random: Some(randombytes_internal_random),
        stir: Some(randombytes_internal_random_stir),
        uniform: None,
        buf: Some(randombytes_internal_random_buf),
        close: Some(randombytes_internal_random_close),
    };
