//! Translation of `libsodium/randombytes/internal/randombytes_internal_random.c`
//!
//! Reference build: `HAVE_GETENTROPY`, `HAVE_GETPID`, `HAVE_RDRAND` and
//! `HAVE_THREADS_H` are undefined; `TLS` expands to nothing (C99 mode), and
//! Linux selects the `getrandom(2)` syscall path.

use core::ffi::{c_char, c_int, c_long, c_void};

use super::randombytes_implementation;
use crate::plat::{get_errno, set_errno, EAGAIN, EINTR, EIO};
use crate::sodium::core::sodium_misuse;

const CRYPTO_STREAM_CHACHA20_KEYBYTES: usize = 32;
const INTERNAL_RANDOM_BLOCK_SIZE: usize = 32; // crypto_core_hchacha20_OUTPUTBYTES

extern "C" {
    fn syscall(num: c_long, ...) -> c_long;
    fn open(path: *const c_char, oflag: c_int, ...) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn close(fd: c_int) -> c_int;
    fn gettimeofday(tv: *mut Timeval, tz: *mut c_void) -> c_int;

    fn crypto_stream_chacha20(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
    fn crypto_stream_chacha20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn sodium_runtime_has_rdrand() -> c_int;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

const SYS_GETRANDOM: c_long = 318;
const O_RDONLY: c_int = 0;

#[repr(C)]
struct Timeval {
    tv_sec: i64,
    tv_usec: i64,
}

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

fn sodium_hrtime() -> u64 {
    let mut tv = Timeval { tv_sec: 0, tv_usec: 0 };
    unsafe {
        if gettimeofday(&mut tv, core::ptr::null_mut()) != 0 {
            sodium_misuse();
        }
    }
    (tv.tv_sec as u64) * 1000000u64 + tv.tv_usec as u64
}

unsafe fn _randombytes_linux_getrandom(buf: *mut c_void, size: usize) -> c_int {
    let mut readnb: c_int;
    loop {
        readnb = syscall(SYS_GETRANDOM, buf, size as c_int, 0 as c_int) as c_int;
        if !(readnb < 0 && (get_errno() == EINTR || get_errno() == EAGAIN)) {
            break;
        }
    }

    (readnb == size as c_int) as c_int - 1
}

unsafe fn randombytes_linux_getrandom(buf_: *mut c_void, mut size: usize) -> c_int {
    let mut buf = buf_ as *mut u8;
    let mut chunk_size: usize = 256;

    loop {
        if size < chunk_size {
            chunk_size = size;
        }
        if _randombytes_linux_getrandom(buf as *mut c_void, chunk_size) != 0 {
            return -1;
        }
        size -= chunk_size;
        buf = buf.add(chunk_size);
        if size == 0 {
            break;
        }
    }

    0
}

unsafe fn randombytes_internal_random_random_dev_open() -> c_int {
    let devices: [*const c_char; 2] = [
        b"/dev/urandom\0".as_ptr() as *const c_char,
        b"/dev/random\0".as_ptr() as *const c_char,
    ];
    let mut idx = 0usize;
    while idx < devices.len() {
        let fd = open(devices[idx], O_RDONLY);
        if fd != -1 {
            return fd;
        } else if get_errno() == EINTR {
            continue;
        }
        idx += 1;
    }
    set_errno(EIO);
    -1
}

unsafe fn safe_read(fd: c_int, buf_: *mut c_void, mut size: usize) -> isize {
    let mut buf = buf_ as *mut u8;
    let mut readnb: isize;
    loop {
        loop {
            readnb = read(fd, buf as *mut c_void, size);
            if !(readnb < 0 && (get_errno() == EINTR || get_errno() == EAGAIN)) {
                break;
            }
        }
        if readnb < 0 {
            return readnb;
        }
        if readnb == 0 {
            break;
        }
        size -= readnb as usize;
        buf = buf.add(readnb as usize);
        if size == 0 {
            break;
        }
    }

    buf.offset_from(buf_ as *mut u8) as isize
}

unsafe fn randombytes_internal_random_init() {
    let errno_save = get_errno();

    GLOBAL.rdrand_available = sodium_runtime_has_rdrand();
    GLOBAL.getentropy_available = 0;
    GLOBAL.getrandom_available = 0;

    {
        let mut fodder = [0u8; 16];
        if randombytes_linux_getrandom(fodder.as_mut_ptr() as *mut c_void, 16) == 0 {
            GLOBAL.getrandom_available = 1;
            set_errno(errno_save);
            return;
        }
    }

    GLOBAL.random_data_source_fd = randombytes_internal_random_random_dev_open();
    if GLOBAL.random_data_source_fd == -1 {
        sodium_misuse();
    }
    set_errno(errno_save);
}

extern "C" fn randombytes_internal_random_stir() {
    unsafe {
        STREAM.nonce = sodium_hrtime();
        core::ptr::write_bytes(STREAM.rnd32.as_mut_ptr(), 0, STREAM.rnd32.len());
        STREAM.rnd32_outleft = 0;
        if GLOBAL.initialized == 0 {
            randombytes_internal_random_init();
            GLOBAL.initialized = 1;
        }

        if GLOBAL.getrandom_available != 0 {
            if randombytes_linux_getrandom(
                STREAM.key.as_mut_ptr() as *mut c_void,
                CRYPTO_STREAM_CHACHA20_KEYBYTES,
            ) != 0
            {
                sodium_misuse();
            }
        } else if GLOBAL.random_data_source_fd == -1
            || safe_read(
                GLOBAL.random_data_source_fd,
                STREAM.key.as_mut_ptr() as *mut c_void,
                CRYPTO_STREAM_CHACHA20_KEYBYTES,
            ) != CRYPTO_STREAM_CHACHA20_KEYBYTES as isize
        {
            sodium_misuse();
        }

        STREAM.initialized = 1;
    }
}

unsafe fn randombytes_internal_random_stir_if_needed() {
    if STREAM.initialized == 0 {
        randombytes_internal_random_stir();
    }
}

extern "C" fn randombytes_internal_random_close() -> c_int {
    let mut ret: c_int = -1;

    unsafe {
        if GLOBAL.getrandom_available != 0 {
            ret = 0;
        }
        sodium_memzero(
            core::ptr::addr_of_mut!(STREAM) as *mut c_void,
            core::mem::size_of::<InternalRandom>(),
        );
    }
    ret
}

/// `HAVE_RDRAND` is undefined: the body compiles away.
fn randombytes_internal_random_xorhwrand() {}

unsafe fn randombytes_internal_random_xorkey(mix: *const u8) {
    for i in 0..CRYPTO_STREAM_CHACHA20_KEYBYTES {
        STREAM.key[i] ^= *mix.add(i);
    }
}

unsafe extern "C" fn randombytes_internal_random_buf(buf: *mut c_void, size: usize) {
    randombytes_internal_random_stir_if_needed();
    crypto_stream_chacha20(
        buf as *mut u8,
        size as u64,
        core::ptr::addr_of!(STREAM.nonce) as *const u8,
        STREAM.key.as_ptr(),
    );
    let size_bytes = size.to_ne_bytes();
    for i in 0..core::mem::size_of::<usize>() {
        STREAM.key[i] ^= size_bytes[i];
    }
    randombytes_internal_random_xorhwrand();
    STREAM.nonce = STREAM.nonce.wrapping_add(1);
    let key_ptr = STREAM.key.as_mut_ptr();
    crypto_stream_chacha20_xor(
        key_ptr,
        key_ptr as *const u8,
        CRYPTO_STREAM_CHACHA20_KEYBYTES as u64,
        core::ptr::addr_of!(STREAM.nonce) as *const u8,
        key_ptr as *const u8,
    );
}

extern "C" fn randombytes_internal_random() -> u32 {
    unsafe {
        if STREAM.rnd32_outleft == 0 {
            randombytes_internal_random_stir_if_needed();
            crypto_stream_chacha20(
                STREAM.rnd32.as_mut_ptr(),
                STREAM.rnd32.len() as u64,
                core::ptr::addr_of!(STREAM.nonce) as *const u8,
                STREAM.key.as_ptr(),
            );
            STREAM.rnd32_outleft = STREAM.rnd32.len() - CRYPTO_STREAM_CHACHA20_KEYBYTES;
            randombytes_internal_random_xorhwrand();
            let off = STREAM.rnd32_outleft;
            randombytes_internal_random_xorkey(STREAM.rnd32.as_ptr().add(off));
            core::ptr::write_bytes(
                STREAM.rnd32.as_mut_ptr().add(off),
                0,
                CRYPTO_STREAM_CHACHA20_KEYBYTES,
            );
            STREAM.nonce = STREAM.nonce.wrapping_add(1);
        }
        STREAM.rnd32_outleft -= 4;
        let off = STREAM.rnd32_outleft;
        let mut val_bytes = [0u8; 4];
        core::ptr::copy_nonoverlapping(STREAM.rnd32.as_ptr().add(off), val_bytes.as_mut_ptr(), 4);
        core::ptr::write_bytes(STREAM.rnd32.as_mut_ptr().add(off), 0, 4);

        u32::from_ne_bytes(val_bytes)
    }
}

extern "C" fn randombytes_internal_implementation_name() -> *const c_char {
    b"internal\0".as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub static randombytes_internal_implementation: randombytes_implementation =
    randombytes_implementation {
        implementation_name: Some(randombytes_internal_implementation_name),
        random: Some(randombytes_internal_random),
        stir: Some(randombytes_internal_random_stir),
        uniform: None,
        buf: Some(randombytes_internal_random_buf),
        close: Some(randombytes_internal_random_close),
    };
