//! Rust translation of `randombytes/internal/randombytes_internal_random.c`.
//!
//! Reference build has none of `HAVE_GETENTROPY`, `HAVE_GETRANDOM`,
//! `HAVE_RDRAND`, `HAVE_SAFE_ARC4RANDOM`, `HAVE_GETPID`,
//! `HAVE_COMMONCRYPTO_COMMONRANDOM_H` defined. `__linux__` is defined and
//! glibc headers define `SYS_getrandom`/`__NR_getrandom`, so
//! `HAVE_LINUX_COMPATIBLE_GETRANDOM` is active (`getrandom(B,S,F)` expands to
//! `syscall(SYS_getrandom, ...)`), which is the branch selected in
//! `randombytes_internal_random_init`. `BLOCK_ON_DEV_RANDOM` is active
//! (`__linux__` defined, `NO_BLOCKING_RANDOM_POLL` not defined). The `TLS`
//! macro resolves to nothing (no `_Thread_local`) under this configuration.

use core::ffi::{c_char, c_int, c_void};

use crate::csys::{
    close, errno, open, poll, read, set_errno, syscall, EAGAIN, EINTR, EIO, O_RDONLY, SYS_getrandom,
};
use crate::types::randombytes_implementation;

extern "C" {
    fn sodium_misuse() -> !;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_runtime_has_rdrand() -> c_int;
    fn crypto_stream_chacha20(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
    fn crypto_stream_chacha20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
}

// ---- extra local libc declarations not present in `crate::csys` ----

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: i16,
    revents: i16,
}

const POLLIN: i16 = 0x001;

// x86_64 glibc `struct stat` layout (144 bytes); we only need `st_mode`.
#[repr(C)]
struct StatBuf {
    st_dev: u64,
    st_ino: u64,
    st_nlink: u64,
    st_mode: u32,
    st_uid: u32,
    st_gid: u32,
    __pad0: i32,
    st_rdev: u64,
    st_size: i64,
    st_blksize: i64,
    st_blocks: i64,
    st_atime: i64,
    st_atime_nsec: i64,
    st_mtime: i64,
    st_mtime_nsec: i64,
    st_ctime: i64,
    st_ctime_nsec: i64,
    __glibc_reserved: [i64; 3],
}

const S_IFMT: u32 = 0o170000;
const S_IFCHR: u32 = 0o020000;

const F_GETFD: c_int = 1;
const F_SETFD: c_int = 2;
const FD_CLOEXEC: c_int = 1;

#[repr(C)]
struct TimeVal {
    tv_sec: i64,
    tv_usec: i64,
}

extern "C" {
    fn fstat(fd: c_int, buf: *mut StatBuf) -> c_int;
    fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    fn gettimeofday(tv: *mut TimeVal, tz: *mut c_void) -> c_int;
}

type ssize_t = isize;
type size_t = usize;

/// The reference build compiles with an empty `CMAKE_BUILD_TYPE`, i.e. **without**
/// `-DNDEBUG`, so the C `assert()`s in this file are live: a failing one calls
/// `abort()` (SIGABRT). Reproduce that observable behaviour.
#[inline(always)]
unsafe fn c_assert(cond: bool) {
    if !cond {
        crate::csys::abort();
    }
}

const CRYPTO_STREAM_CHACHA20_KEYBYTES: usize = 32;
const INTERNAL_RANDOM_BLOCK_SIZE: usize = 32; // crypto_core_hchacha20_OUTPUTBYTES

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
    rnd32_outleft: size_t,
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

// The C `TLS` macro expands to nothing in this configuration (no
// `_Thread_local`), so this is a plain (non-thread-local) global.
static mut STREAM: InternalRandom = InternalRandom {
    initialized: 0,
    rnd32_outleft: 0,
    key: [0u8; CRYPTO_STREAM_CHACHA20_KEYBYTES],
    rnd32: [0u8; 16 * INTERNAL_RANDOM_BLOCK_SIZE],
    nonce: 0,
};

unsafe fn sodium_hrtime() -> u64 {
    let mut tv = TimeVal { tv_sec: 0, tv_usec: 0 };
    if gettimeofday(&mut tv, core::ptr::null_mut()) != 0 {
        sodium_misuse();
    }
    (tv.tv_sec as u64).wrapping_mul(1_000_000).wrapping_add(tv.tv_usec as u64)
}

unsafe fn _randombytes_linux_getrandom(buf: *mut c_void, size: size_t) -> c_int {
    c_assert(size <= 256);
    let mut readnb: c_int;
    loop {
        readnb = syscall(SYS_getrandom, buf, size as c_int, 0) as c_int;
        if !(readnb < 0 && (errno() == EINTR || errno() == EAGAIN)) {
            break;
        }
    }
    ((readnb == size as c_int) as c_int) - 1
}

unsafe fn randombytes_linux_getrandom(buf_: *mut c_void, mut size: size_t) -> c_int {
    let mut buf = buf_ as *mut u8;
    let mut chunk_size: size_t = 256;

    loop {
        if size < chunk_size {
            chunk_size = size;
            c_assert(chunk_size > 0);
        }
        if _randombytes_linux_getrandom(buf as *mut c_void, chunk_size) != 0 {
            return -1;
        }
        size -= chunk_size;
        buf = buf.add(chunk_size);
        if !(size > 0) {
            break;
        }
    }

    0
}

unsafe fn randombytes_block_on_dev_random() -> c_int {
    let path = b"/dev/random\0";
    let fd = open(path.as_ptr() as *const c_char, O_RDONLY);
    if fd == -1 {
        return 0;
    }
    let mut pfd = PollFd {
        fd,
        events: POLLIN,
        revents: 0,
    };
    let mut pret: c_int;
    loop {
        pret = poll(&mut pfd as *mut PollFd as *mut c_void, 1, -1);
        if !(pret < 0 && (errno() == EINTR || errno() == EAGAIN)) {
            break;
        }
    }
    if pret != 1 {
        let _ = close(fd);
        set_errno(EIO);
        return -1;
    }
    close(fd)
}

unsafe fn randombytes_internal_random_random_dev_open() -> c_int {
    let devices: [*const c_char; 3] = [
        b"/dev/urandom\0".as_ptr() as *const c_char,
        b"/dev/random\0".as_ptr() as *const c_char,
        core::ptr::null(),
    ];
    let mut device: *const *const c_char = devices.as_ptr();
    let mut fd: c_int;
    let mut st: StatBuf = core::mem::zeroed();

    if randombytes_block_on_dev_random() != 0 {
        return -1;
    }

    loop {
        fd = open(*device, O_RDONLY);
        if fd != -1 {
            if fstat(fd, &mut st) == 0 && (st.st_mode & S_IFMT) == S_IFCHR {
                let _ = fcntl(fd, F_SETFD, fcntl(fd, F_GETFD) | FD_CLOEXEC);
                return fd;
            }
            let _ = close(fd);
        } else if errno() == EINTR {
            continue;
        }
        device = device.add(1);
        if (*device).is_null() {
            break;
        }
    }

    set_errno(EIO);
    -1
}

unsafe fn safe_read(fd: c_int, buf_: *mut c_void, mut size: size_t) -> ssize_t {
    let mut buf = buf_ as *mut u8;
    let orig = buf_ as *mut u8;
    let mut readnb: ssize_t;

    c_assert(size > 0);
    c_assert(size <= usize::MAX / 2 - 1); // SSIZE_MAX
    loop {
        loop {
            readnb = read(fd, buf as *mut c_void, size);
            if !(readnb < 0 && (errno() == EINTR || errno() == EAGAIN)) {
                break;
            }
        }
        if readnb < 0 {
            return readnb;
        }
        if readnb == 0 {
            break;
        }
        size -= readnb as size_t;
        buf = buf.add(readnb as usize);
        if !(size > 0) {
            break;
        }
    }

    (buf as isize - orig as isize) as ssize_t
}

#[allow(unreachable_code)]
unsafe fn randombytes_internal_random_init() {
    let errno_save = errno();

    GLOBAL.rdrand_available = sodium_runtime_has_rdrand();
    GLOBAL.getentropy_available = 0;
    GLOBAL.getrandom_available = 0;

    {
        let mut fodder = [0u8; 16];
        if randombytes_linux_getrandom(fodder.as_mut_ptr() as *mut c_void, fodder.len()) == 0 {
            GLOBAL.getrandom_available = 1;
            set_errno(errno_save);
            return;
        }
    }

    c_assert((GLOBAL.getentropy_available | GLOBAL.getrandom_available) == 0);
    GLOBAL.random_data_source_fd = randombytes_internal_random_random_dev_open();
    if GLOBAL.random_data_source_fd == -1 {
        sodium_misuse();
    }
    set_errno(errno_save);
    return;

    // Unreachable in this configuration: `#ifndef HAVE_SAFE_ARC4RANDOM` block
    // after the `#if !defined(NONEXISTENT_DEV_RANDOM)` branch's `return;`.
    sodium_misuse();
}

unsafe fn randombytes_internal_random_stir() {
    STREAM.nonce = sodium_hrtime();
    c_assert(STREAM.nonce != 0);
    let rnd32 = (&raw mut STREAM.rnd32) as *mut u8;
    crate::csys::memset(rnd32 as *mut c_void, 0, 16 * INTERNAL_RANDOM_BLOCK_SIZE);
    STREAM.rnd32_outleft = 0;
    if GLOBAL.initialized == 0 {
        randombytes_internal_random_init();
        GLOBAL.initialized = 1;
    }

    if GLOBAL.getrandom_available != 0 {
        let key = (&raw mut STREAM.key) as *mut u8;
        if randombytes_linux_getrandom(key as *mut c_void, CRYPTO_STREAM_CHACHA20_KEYBYTES) != 0 {
            sodium_misuse();
        }
    }

    STREAM.initialized = 1;
}

unsafe fn randombytes_internal_random_stir_if_needed() {
    if STREAM.initialized == 0 {
        randombytes_internal_random_stir();
    }
}

unsafe fn randombytes_internal_random_close() -> c_int {
    let mut ret: c_int = -1;

    if GLOBAL.getrandom_available != 0 {
        ret = 0;
    }

    sodium_memzero(
        (&raw mut STREAM) as *mut c_void,
        core::mem::size_of::<InternalRandom>(),
    );

    ret
}

unsafe fn randombytes_internal_random_xorhwrand() {
    // HAVE_RDRAND is not defined in this configuration: no-op.
}

#[inline]
unsafe fn randombytes_internal_random_xorkey(mix: *const u8) {
    let key = (&raw mut STREAM.key) as *mut u8;
    for i in 0..CRYPTO_STREAM_CHACHA20_KEYBYTES {
        *key.add(i) ^= *mix.add(i);
    }
}

unsafe fn randombytes_internal_random_buf(buf: *mut c_void, size: size_t) {
    randombytes_internal_random_stir_if_needed();
    // COMPILER_ASSERT(sizeof stream.nonce == crypto_stream_chacha20_NONCEBYTES);

    let key_ptr = (&raw mut STREAM.key) as *mut u8;
    let key_len = CRYPTO_STREAM_CHACHA20_KEYBYTES;
    let mut nonce_ptr = (&raw mut STREAM.nonce) as *const u8;

    let ret = crypto_stream_chacha20(buf as *mut u8, size as u64, nonce_ptr, key_ptr);
    c_assert(ret == 0);

    let size_bytes = size.to_ne_bytes();
    for i in 0..core::mem::size_of::<size_t>() {
        *key_ptr.add(i) ^= size_bytes[i];
    }
    randombytes_internal_random_xorhwrand();
    STREAM.nonce = STREAM.nonce.wrapping_add(1);
    nonce_ptr = (&raw mut STREAM.nonce) as *const u8;
    crypto_stream_chacha20_xor(key_ptr, key_ptr, key_len as u64, nonce_ptr, key_ptr);
}

unsafe fn randombytes_internal_random() -> u32 {
    let val: u32;
    let key_len = CRYPTO_STREAM_CHACHA20_KEYBYTES;
    let rnd32_len = 16 * INTERNAL_RANDOM_BLOCK_SIZE;

    // COMPILER_ASSERT(sizeof stream.rnd32 >= (sizeof stream.key) + (sizeof val));
    // COMPILER_ASSERT(((sizeof stream.rnd32) - (sizeof stream.key)) % sizeof val == 0);
    if STREAM.rnd32_outleft == 0 {
        randombytes_internal_random_stir_if_needed();
        let rnd32_ptr = (&raw mut STREAM.rnd32) as *mut u8;
        let nonce_ptr = (&raw mut STREAM.nonce) as *const u8;
        let key_ptr = (&raw mut STREAM.key) as *const u8;
        let ret = crypto_stream_chacha20(rnd32_ptr, rnd32_len as u64, nonce_ptr, key_ptr);
        c_assert(ret == 0);
        STREAM.rnd32_outleft = rnd32_len - key_len;
        randombytes_internal_random_xorhwrand();
        let mix_off = STREAM.rnd32_outleft;
        randombytes_internal_random_xorkey(rnd32_ptr.add(mix_off));
        crate::csys::memset(rnd32_ptr.add(mix_off) as *mut c_void, 0, key_len);
        STREAM.nonce = STREAM.nonce.wrapping_add(1);
    }
    STREAM.rnd32_outleft -= core::mem::size_of::<u32>();
    let off = STREAM.rnd32_outleft;
    let rnd32_ptr = (&raw mut STREAM.rnd32) as *mut u8;
    let src = rnd32_ptr.add(off);
    let mut v: u32 = 0;
    crate::csys::memcpy(
        &mut v as *mut u32 as *mut c_void,
        src as *const c_void,
        core::mem::size_of::<u32>(),
    );
    crate::csys::memset(src as *mut c_void, 0, core::mem::size_of::<u32>());
    val = v;

    val
}

unsafe extern "C" fn randombytes_internal_implementation_name() -> *const c_char {
    b"internal\0".as_ptr() as *const c_char
}

unsafe extern "C" fn randombytes_internal_random_extern() -> u32 {
    randombytes_internal_random()
}

unsafe extern "C" fn randombytes_internal_random_stir_extern() {
    randombytes_internal_random_stir();
}

unsafe extern "C" fn randombytes_internal_random_buf_extern(buf: *mut c_void, size: size_t) {
    randombytes_internal_random_buf(buf, size);
}

unsafe extern "C" fn randombytes_internal_random_close_extern() -> c_int {
    randombytes_internal_random_close()
}

#[no_mangle]
pub static randombytes_internal_implementation: randombytes_implementation =
    randombytes_implementation {
        implementation_name: Some(randombytes_internal_implementation_name),
        random: Some(randombytes_internal_random_extern),
        stir: Some(randombytes_internal_random_stir_extern),
        uniform: None,
        buf: Some(randombytes_internal_random_buf_extern),
        close: Some(randombytes_internal_random_close_extern),
    };
