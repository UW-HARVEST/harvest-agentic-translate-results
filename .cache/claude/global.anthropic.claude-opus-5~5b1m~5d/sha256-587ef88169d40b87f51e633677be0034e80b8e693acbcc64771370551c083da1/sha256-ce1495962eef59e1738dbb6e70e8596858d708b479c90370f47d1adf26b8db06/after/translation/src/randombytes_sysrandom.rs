//! Rust translation of `randombytes/sysrandom/randombytes_sysrandom.c`.
//!
//! Reference build has none of `HAVE_SYS_RANDOM_H`, `HAVE_GETRANDOM`,
//! `HAVE_SAFE_ARC4RANDOM` defined, but `__linux__` is defined and the glibc
//! headers define `SYS_getrandom`/`__NR_getrandom`, so
//! `HAVE_LINUX_COMPATIBLE_GETRANDOM` is active via the `getrandom(B,S,F)` ->
//! `syscall(SYS_getrandom, ...)` macro. `NO_BLOCKING_RANDOM_POLL` is not
//! defined and `__linux__` is defined, so `BLOCK_ON_DEV_RANDOM` is active.

use core::ffi::{c_char, c_int, c_void};

use crate::csys::{
    close, errno, open, poll, read, set_errno, syscall, EAGAIN, EINTR, EIO, O_RDONLY, SYS_getrandom,
};
use crate::types::randombytes_implementation;

extern "C" {
    fn sodium_misuse() -> !;
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

extern "C" {
    fn fstat(fd: c_int, buf: *mut StatBuf) -> c_int;
    fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
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

#[repr(C)]
struct SysRandom {
    random_data_source_fd: c_int,
    initialized: c_int,
    getrandom_available: c_int,
}

static mut STREAM: SysRandom = SysRandom {
    random_data_source_fd: -1,
    initialized: 0,
    getrandom_available: 0,
};

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

unsafe fn randombytes_sysrandom_random_dev_open() -> c_int {
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

unsafe fn randombytes_sysrandom_init() {
    let errno_save = errno();

    let mut fodder = [0u8; 16];
    if randombytes_linux_getrandom(fodder.as_mut_ptr() as *mut c_void, fodder.len()) == 0 {
        STREAM.getrandom_available = 1;
        set_errno(errno_save);
        return;
    }
    STREAM.getrandom_available = 0;

    STREAM.random_data_source_fd = randombytes_sysrandom_random_dev_open();
    if STREAM.random_data_source_fd == -1 {
        sodium_misuse();
    }
    set_errno(errno_save);
}

unsafe fn randombytes_sysrandom_stir() {
    if STREAM.initialized == 0 {
        randombytes_sysrandom_init();
        STREAM.initialized = 1;
    }
}

unsafe fn randombytes_sysrandom_stir_if_needed() {
    if STREAM.initialized == 0 {
        randombytes_sysrandom_stir();
    }
}

unsafe fn randombytes_sysrandom_close() -> c_int {
    let mut ret: c_int = -1;

    if STREAM.random_data_source_fd != -1 && close(STREAM.random_data_source_fd) == 0 {
        STREAM.random_data_source_fd = -1;
        STREAM.initialized = 0;
        ret = 0;
    }

    if STREAM.getrandom_available != 0 {
        ret = 0;
    }

    ret
}

unsafe fn randombytes_sysrandom_buf(buf: *mut c_void, size: size_t) {
    randombytes_sysrandom_stir_if_needed();
    if STREAM.getrandom_available != 0 {
        if randombytes_linux_getrandom(buf, size) != 0 {
            sodium_misuse();
        }
        return;
    }

    if STREAM.random_data_source_fd == -1
        || safe_read(STREAM.random_data_source_fd, buf, size) != size as ssize_t
    {
        sodium_misuse();
    }
}

unsafe extern "C" fn randombytes_sysrandom() -> u32 {
    let mut r: u32 = 0;
    randombytes_sysrandom_buf(&mut r as *mut u32 as *mut c_void, core::mem::size_of::<u32>());
    r
}

unsafe extern "C" fn randombytes_sysrandom_implementation_name() -> *const c_char {
    b"sysrandom\0".as_ptr() as *const c_char
}

unsafe extern "C" fn randombytes_sysrandom_stir_extern() {
    randombytes_sysrandom_stir();
}

unsafe extern "C" fn randombytes_sysrandom_buf_extern(buf: *mut c_void, size: size_t) {
    randombytes_sysrandom_buf(buf, size);
}

unsafe extern "C" fn randombytes_sysrandom_close_extern() -> c_int {
    randombytes_sysrandom_close()
}

#[no_mangle]
pub static randombytes_sysrandom_implementation: randombytes_implementation =
    randombytes_implementation {
        implementation_name: Some(randombytes_sysrandom_implementation_name),
        random: Some(randombytes_sysrandom),
        stir: Some(randombytes_sysrandom_stir_extern),
        uniform: None,
        buf: Some(randombytes_sysrandom_buf_extern),
        close: Some(randombytes_sysrandom_close_extern),
    };
