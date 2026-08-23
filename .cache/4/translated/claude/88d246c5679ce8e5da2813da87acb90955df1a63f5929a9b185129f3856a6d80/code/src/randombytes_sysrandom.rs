//! Translation of `randombytes/sysrandom/randombytes_sysrandom.c`.
//!
//! Reference build: Linux/glibc x86-64, no `config.h`.
//!
//! Surviving preprocessor configuration:
//!   * `_WIN32`                          – undefined
//!   * `HAVE_SAFE_ARC4RANDOM`            – undefined (not OpenBSD/CloudABI/wasi)
//!   * `HAVE_SYS_RANDOM_H` / `HAVE_GETRANDOM` – undefined, but `__linux__` is
//!     defined and `<sys/syscall.h>` provides `SYS_getrandom` (318 on x86-64),
//!     so `getrandom(B, S, F)` expands to `syscall(318, B, (int) S, F)` and
//!     **`HAVE_LINUX_COMPATIBLE_GETRANDOM` is defined**.
//!   * `NO_BLOCKING_RANDOM_POLL`         – undefined and `__linux__` defined, so
//!     **`BLOCK_ON_DEV_RANDOM` is defined** (`/dev/random` is polled first).
//!   * `USE_BLOCKING_RANDOM`             – undefined, so `/dev/urandom` is tried first.
//!   * `NDEBUG`                          – **defined** (the reference build uses
//!     `-O3 -DNDEBUG -fPIC -std=gnu99`), so every `assert()` collapses to
//!     `((void) (0))`; the predicates are kept as comments only.

use core::ffi::{c_char, c_int, c_long, c_short, c_ulong, c_void};
use core::mem::MaybeUninit;

/* ------------------------------------------------------------------ */
/*  include/sodium/randombytes.h                                      */
/* ------------------------------------------------------------------ */

/// `typedef struct randombytes_implementation { ... } randombytes_implementation;`
#[repr(C)]
pub struct randombytes_implementation {
    pub implementation_name: Option<extern "C" fn() -> *const c_char>,
    pub random: Option<extern "C" fn() -> u32>,
    pub stir: Option<extern "C" fn()>,
    pub uniform: Option<extern "C" fn(upper_bound: u32) -> u32>,
    pub buf: Option<extern "C" fn(buf: *mut c_void, size: usize)>,
    pub close: Option<extern "C" fn() -> c_int>,
}

/* ------------------------------------------------------------------ */
/*  libc                                                              */
/* ------------------------------------------------------------------ */

const O_RDONLY: c_int = 0o0;
const F_GETFD: c_int = 1;
const F_SETFD: c_int = 2;
const FD_CLOEXEC: c_int = 1;
const POLLIN: c_short = 0x001;
const EINTR: c_int = 4;
const EAGAIN: c_int = 11;
const EIO: c_int = 5;
const S_IFMT: u32 = 0o170000;
const S_IFCHR: u32 = 0o020000;
const SSIZE_MAX: usize = 0x7fff_ffff_ffff_ffff;
const SYS_getrandom: c_long = 318;

#[repr(C)]
struct pollfd {
    fd: c_int,
    events: c_short,
    revents: c_short,
}

/// glibc `struct stat` for x86-64 (144 bytes).
#[repr(C)]
struct stat {
    st_dev: u64,
    st_ino: u64,
    st_nlink: u64,
    st_mode: u32,
    st_uid: u32,
    st_gid: u32,
    __pad0: c_int,
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

extern "C" {
    fn open(path: *const c_char, oflag: c_int, ...) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, nbyte: usize) -> isize;
    fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    fn fstat(fd: c_int, buf: *mut stat) -> c_int;
    fn poll(fds: *mut pollfd, nfds: c_ulong, timeout: c_int) -> c_int;
    fn syscall(number: c_long, ...) -> c_long;
    fn __errno_location() -> *mut c_int;

    /* sodium/core.c */
    fn sodium_misuse() -> !;
}

#[inline(always)]
unsafe fn errno() -> c_int {
    *__errno_location()
}

#[inline(always)]
unsafe fn set_errno(v: c_int) {
    *__errno_location() = v;
}

/* ------------------------------------------------------------------ */
/*  typedef struct SysRandom_ { ... } SysRandom;                      */
/* ------------------------------------------------------------------ */

#[repr(C)]
struct SysRandom {
    random_data_source_fd: c_int,
    initialized: c_int,
    getrandom_available: c_int,
}

static mut stream: SysRandom = SysRandom {
    random_data_source_fd: -1,
    initialized: 0,
    getrandom_available: 0,
};

/* ------------------------------------------------------------------ */

/// `static ssize_t safe_read(const int fd, void * const buf_, size_t size)`
unsafe fn safe_read(fd: c_int, buf_: *mut c_void, mut size: usize) -> isize {
    let mut buf: *mut u8 = buf_ as *mut u8;
    let mut readnb: isize;

    /* assert(size > (size_t) 0U);  -> no-op under NDEBUG */
    /* assert(size <= SSIZE_MAX);   -> no-op under NDEBUG */
    loop {
        loop {
            readnb = read(fd, buf as *mut c_void, size);
            if !(readnb < 0 && (errno() == EINTR || errno() == EAGAIN)) {
                break;
            }
        } /* LCOV_EXCL_LINE */
        if readnb < 0 {
            return readnb; /* LCOV_EXCL_LINE */
        }
        if readnb == 0 {
            break; /* LCOV_EXCL_LINE */
        }
        size -= readnb as usize;
        buf = buf.add(readnb as usize);
        if !(size > 0) {
            break;
        }
    }

    (buf as isize).wrapping_sub(buf_ as isize)
}

/// `static int randombytes_block_on_dev_random(void)`  (BLOCK_ON_DEV_RANDOM)
unsafe fn randombytes_block_on_dev_random() -> c_int {
    let mut pfd = MaybeUninit::<pollfd>::uninit();
    let fd: c_int;
    let mut pret: c_int;

    fd = open(c"/dev/random".as_ptr(), O_RDONLY);
    if fd == -1 {
        return 0;
    }
    let p = pfd.as_mut_ptr();
    (*p).fd = fd;
    (*p).events = POLLIN;
    (*p).revents = 0;
    loop {
        pret = poll(p, 1, -1);
        if !(pret < 0 && (errno() == EINTR || errno() == EAGAIN)) {
            break;
        }
    }
    if pret != 1 {
        close(fd);
        set_errno(EIO);
        return -1;
    }
    close(fd)
}

/// `static int randombytes_sysrandom_random_dev_open(void)`
unsafe fn randombytes_sysrandom_random_dev_open() -> c_int {
    /* LCOV_EXCL_START */
    let mut st = MaybeUninit::<stat>::uninit();
    let devices: [*const c_char; 3] = [
        c"/dev/urandom".as_ptr(),
        c"/dev/random".as_ptr(),
        core::ptr::null(),
    ];
    let mut device: *const *const c_char = devices.as_ptr();
    let mut fd: c_int;

    if randombytes_block_on_dev_random() != 0 {
        return -1;
    }
    loop {
        fd = open(*device, O_RDONLY);
        if fd != -1 {
            if fstat(fd, st.as_mut_ptr()) == 0
                && ((*st.as_ptr()).st_mode & S_IFMT) == S_IFCHR
            {
                fcntl(fd, F_SETFD, fcntl(fd, F_GETFD) | FD_CLOEXEC);
                return fd;
            }
            close(fd);
        } else if errno() == EINTR {
            /* `continue` inside a do/while jumps straight to the controlling
             * expression, i.e. the same device is retried. */
            if (*device).is_null() {
                break;
            }
            continue;
        }
        device = device.add(1);
        if (*device).is_null() {
            break;
        }
    }

    set_errno(EIO);
    -1
    /* LCOV_EXCL_STOP */
}

/// `static int _randombytes_linux_getrandom(void * const buf, const size_t size)`
unsafe fn _randombytes_linux_getrandom(buf: *mut c_void, size: usize) -> c_int {
    let mut readnb: c_int;

    /* assert(size <= 256U);  -> no-op under NDEBUG */
    loop {
        readnb = syscall(SYS_getrandom, buf, size as c_int, 0 as c_int) as c_int;
        if !(readnb < 0 && (errno() == EINTR || errno() == EAGAIN)) {
            break;
        }
    }

    (readnb == size as c_int) as c_int - 1
}

/// `static int randombytes_linux_getrandom(void * const buf_, size_t size)`
unsafe fn randombytes_linux_getrandom(buf_: *mut c_void, mut size: usize) -> c_int {
    let mut buf: *mut u8 = buf_ as *mut u8;
    let mut chunk_size: usize = 256;

    loop {
        if size < chunk_size {
            chunk_size = size;
            /* assert(chunk_size > (size_t) 0U);  -> no-op under NDEBUG */
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

/// `static void randombytes_sysrandom_init(void)`
unsafe fn randombytes_sysrandom_init() {
    let errno_save: c_int = errno();

    /* HAVE_LINUX_COMPATIBLE_GETRANDOM */
    {
        let mut fodder = [0u8; 16];

        if randombytes_linux_getrandom(fodder.as_mut_ptr() as *mut c_void, 16) == 0 {
            stream.getrandom_available = 1;
            set_errno(errno_save);
            return;
        }
        stream.getrandom_available = 0;
    }

    stream.random_data_source_fd = randombytes_sysrandom_random_dev_open();
    if stream.random_data_source_fd == -1 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    set_errno(errno_save);
}

/// `static void randombytes_sysrandom_stir(void)`
extern "C" fn randombytes_sysrandom_stir() {
    unsafe {
        if stream.initialized == 0 {
            randombytes_sysrandom_init();
            stream.initialized = 1;
        }
    }
}

/// `static void randombytes_sysrandom_stir_if_needed(void)`
unsafe fn randombytes_sysrandom_stir_if_needed() {
    if stream.initialized == 0 {
        randombytes_sysrandom_stir();
    }
}

/// `static int randombytes_sysrandom_close(void)`
extern "C" fn randombytes_sysrandom_close() -> c_int {
    unsafe {
        let mut ret: c_int = -1;

        if stream.random_data_source_fd != -1 && close(stream.random_data_source_fd) == 0 {
            stream.random_data_source_fd = -1;
            stream.initialized = 0;
            ret = 0;
        }
        /* HAVE_LINUX_COMPATIBLE_GETRANDOM */
        if stream.getrandom_available != 0 {
            ret = 0;
        }
        ret
    }
}

/// `static void randombytes_sysrandom_buf(void * const buf, const size_t size)`
extern "C" fn randombytes_sysrandom_buf(buf: *mut c_void, size: usize) {
    unsafe {
        randombytes_sysrandom_stir_if_needed();
        /* HAVE_LINUX_COMPATIBLE_GETRANDOM */
        if stream.getrandom_available != 0 {
            if randombytes_linux_getrandom(buf, size) != 0 {
                sodium_misuse(); /* LCOV_EXCL_LINE */
            }
            return;
        }
        if stream.random_data_source_fd == -1
            || safe_read(stream.random_data_source_fd, buf, size) != size as isize
        {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
    }
}

/// `static uint32_t randombytes_sysrandom(void)`
extern "C" fn randombytes_sysrandom() -> u32 {
    let mut r: u32 = 0;

    randombytes_sysrandom_buf(&mut r as *mut u32 as *mut c_void, 4);

    r
}

/// `static const char *randombytes_sysrandom_implementation_name(void)`
extern "C" fn randombytes_sysrandom_implementation_name() -> *const c_char {
    c"sysrandom".as_ptr()
}

/* ------------------------------------------------------------------ */
/*  exported data symbol                                              */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub static randombytes_sysrandom_implementation: randombytes_implementation =
    randombytes_implementation {
        implementation_name: Some(randombytes_sysrandom_implementation_name),
        random: Some(randombytes_sysrandom),
        stir: Some(randombytes_sysrandom_stir),
        uniform: None,
        buf: Some(randombytes_sysrandom_buf),
        close: Some(randombytes_sysrandom_close),
    };
