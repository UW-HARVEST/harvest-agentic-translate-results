//! Translation of `randombytes/internal/randombytes_internal_random.c`.
//!
//! Reference build: Linux/glibc x86-64, no `config.h`.
//!
//! Surviving preprocessor configuration:
//!   * `_WIN32`, `__CloudABI__`, `__wasm__`, `__OpenBSD__`   – undefined, hence
//!     neither `NONEXISTENT_DEV_RANDOM` nor `HAVE_SAFE_ARC4RANDOM`.
//!   * `HAVE_COMMONCRYPTO_COMMONRANDOM_H`, `HAVE_GETENTROPY` – undefined.
//!   * `HAVE_SYS_RANDOM_H`/`HAVE_GETRANDOM`                  – undefined, but
//!     `__linux__` is defined and `<sys/syscall.h>` provides `SYS_getrandom`
//!     (318 on x86-64), so `getrandom(B, S, F)` expands to
//!     `syscall(318, B, (int) S, F)` and **`HAVE_LINUX_COMPATIBLE_GETRANDOM`
//!     is defined**.  Consequently the `# elif defined(HAVE_LINUX_COMPATIBLE_GETRANDOM)`
//!     branch of `randombytes_internal_random_stir()` is the one that survives
//!     (there is *no* `safe_read()` fallback in that branch — `safe_read()`
//!     ends up unreferenced, exactly as in C).
//!   * `NO_BLOCKING_RANDOM_POLL` undefined + `__linux__` ⇒ **`BLOCK_ON_DEV_RANDOM`**.
//!   * `USE_BLOCKING_RANDOM`  – undefined, so `/dev/urandom` is tried first.
//!   * `HAVE_GETPID`          – undefined, so `global.pid` does not exist and
//!     `randombytes_internal_random_stir_if_needed()` only tests `initialized`.
//!   * `HAVE_RDRAND`          – undefined, so `..._xorhwrand()` is an empty body.
//!   * The reference build compiles with `-O3 -DNDEBUG -fPIC -std=gnu99`, hence
//!     `__STDC_VERSION__ == 199901L < 201112L` and the `TLS` fallback
//!     `# define TLS` (i.e. **nothing**) is selected: `stream` is a plain
//!     process-wide static, *not* thread-local.  `global` is process-wide too.
//!   * `NDEBUG` is **defined**, so every `assert()` collapses to `((void) (0))`;
//!     the predicates are kept as comments only.

use core::ffi::{c_char, c_int, c_long, c_short, c_ulong, c_ulonglong, c_void};
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

/// `#define INTERNAL_RANDOM_BLOCK_SIZE crypto_core_hchacha20_OUTPUTBYTES` (32U)
const INTERNAL_RANDOM_BLOCK_SIZE: usize = 32;
/// `#define crypto_stream_chacha20_KEYBYTES 32U`
const crypto_stream_chacha20_KEYBYTES: usize = 32;
/// `#define crypto_stream_chacha20_NONCEBYTES 8U`
const crypto_stream_chacha20_NONCEBYTES: usize = 8;

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

#[repr(C)]
struct timeval {
    tv_sec: i64,
    tv_usec: i64,
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
    fn gettimeofday(tv: *mut timeval, tz: *mut c_void) -> c_int;
    fn __errno_location() -> *mut c_int;

    /* sodium/core.c */
    fn sodium_misuse() -> !;
    /* sodium/utils.c */
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    /* sodium/runtime.c */
    fn sodium_runtime_has_rdrand() -> c_int;
    /* crypto_stream/chacha20/stream_chacha20.c */
    fn crypto_stream_chacha20(
        c: *mut u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_chacha20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
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
/*  state                                                             */
/* ------------------------------------------------------------------ */

/// `typedef struct InternalRandomGlobal_ { ... } InternalRandomGlobal;`
/// (`HAVE_GETPID` undefined ⇒ no `pid` member.)
#[repr(C)]
struct InternalRandomGlobal {
    initialized: c_int,
    random_data_source_fd: c_int,
    getentropy_available: c_int,
    getrandom_available: c_int,
    rdrand_available: c_int,
}

/// `typedef struct InternalRandom_ { ... } InternalRandom;`
#[repr(C)]
struct InternalRandom {
    initialized: c_int,
    rnd32_outleft: usize,
    key: [u8; crypto_stream_chacha20_KEYBYTES],
    rnd32: [u8; 16 * INTERNAL_RANDOM_BLOCK_SIZE],
    nonce: u64,
}

static mut global: InternalRandomGlobal = InternalRandomGlobal {
    initialized: 0,
    random_data_source_fd: -1,
    getentropy_available: 0,
    getrandom_available: 0,
    rdrand_available: 0,
};

/* `static TLS InternalRandom stream = { .initialized = 0, .rnd32_outleft = 0 };`
 *
 * `TLS` expands to nothing under `-std=gnu99`, so this is one shared
 * process-wide object (exactly as in the reference `.so`). */
static mut stream: InternalRandom = InternalRandom {
    initialized: 0,
    rnd32_outleft: 0,
    key: [0u8; crypto_stream_chacha20_KEYBYTES],
    rnd32: [0u8; 16 * INTERNAL_RANDOM_BLOCK_SIZE],
    nonce: 0,
};

#[inline(always)]
fn stream_ptr() -> *mut InternalRandom {
    core::ptr::addr_of_mut!(stream)
}

/* ------------------------------------------------------------------ */
/*  Get a high-resolution timestamp, as a uint64_t value               */
/* ------------------------------------------------------------------ */

/// `static uint64_t sodium_hrtime(void)`
unsafe fn sodium_hrtime() -> u64 {
    let mut tv = MaybeUninit::<timeval>::uninit();

    if gettimeofday(tv.as_mut_ptr(), core::ptr::null_mut()) != 0 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    let tv = tv.assume_init();
    (tv.tv_sec as u64)
        .wrapping_mul(1000000u64)
        .wrapping_add(tv.tv_usec as u64)
}

/* ------------------------------------------------------------------ */
/*  Initialize the entropy source                                     */
/* ------------------------------------------------------------------ */

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

/// `static int randombytes_internal_random_random_dev_open(void)`
unsafe fn randombytes_internal_random_random_dev_open() -> c_int {
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
            /* S_ISNAM(x) is 0 here (neither <sys/stat.h> nor __COMPCERT__ define it) */
            if fstat(fd, st.as_mut_ptr()) == 0
                && (false || ((*st.as_ptr()).st_mode & S_IFMT) == S_IFCHR)
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

/// `static ssize_t safe_read(const int fd, void * const buf_, size_t size)`
///
/// Unreferenced in this translation unit for the reference configuration (the
/// `HAVE_LINUX_COMPATIBLE_GETRANDOM` branch of `..._stir()` wins), exactly like
/// in the C build.
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

/// `static void randombytes_internal_random_init(void)`
unsafe fn randombytes_internal_random_init() {
    let errno_save: c_int = errno();

    global.rdrand_available = sodium_runtime_has_rdrand();
    global.getentropy_available = 0;
    global.getrandom_available = 0;

    /* HAVE_LINUX_COMPATIBLE_GETRANDOM */
    {
        let mut fodder = [0u8; 16];

        if randombytes_linux_getrandom(fodder.as_mut_ptr() as *mut c_void, 16) == 0 {
            global.getrandom_available = 1;
            set_errno(errno_save);
            return;
        }
    }
    /* LCOV_EXCL_START */
    /* assert((global.getentropy_available | global.getrandom_available) == 0);
     * -> no-op under NDEBUG */
    global.random_data_source_fd = randombytes_internal_random_random_dev_open();
    if global.random_data_source_fd == -1 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    set_errno(errno_save);
    /* the trailing `sodium_misuse()` of the C source is unreachable here */
    /* LCOV_EXCL_STOP */
}

/* ------------------------------------------------------------------ */
/*  (Re)seed the generator using the entropy source                    */
/* ------------------------------------------------------------------ */

/// `static void randombytes_internal_random_stir(void)`
extern "C" fn randombytes_internal_random_stir() {
    unsafe {
        let s = stream_ptr();

        (*s).nonce = sodium_hrtime();
        /* assert(stream.nonce != (uint64_t) 0U);  -> no-op under NDEBUG */
        core::ptr::write_bytes(
            (*s).rnd32.as_mut_ptr(),
            0u8,
            16 * INTERNAL_RANDOM_BLOCK_SIZE,
        );
        (*s).rnd32_outleft = 0;
        if global.initialized == 0 {
            randombytes_internal_random_init();
            global.initialized = 1;
        }

        /* HAVE_LINUX_COMPATIBLE_GETRANDOM */
        if global.getrandom_available != 0 {
            if randombytes_linux_getrandom(
                (*s).key.as_mut_ptr() as *mut c_void,
                crypto_stream_chacha20_KEYBYTES,
            ) != 0
            {
                sodium_misuse(); /* LCOV_EXCL_LINE */
            }
        }

        (*s).initialized = 1;
    }
}

/* ------------------------------------------------------------------ */
/*  Reseed the generator if it hasn't been initialized yet             */
/* ------------------------------------------------------------------ */

/// `static void randombytes_internal_random_stir_if_needed(void)`
unsafe fn randombytes_internal_random_stir_if_needed() {
    /* !HAVE_GETPID */
    if (*stream_ptr()).initialized == 0 {
        randombytes_internal_random_stir();
    }
}

/* ------------------------------------------------------------------ */
/*  Close the stream, free global resources                            */
/* ------------------------------------------------------------------ */

/// `static int randombytes_internal_random_close(void)`
extern "C" fn randombytes_internal_random_close() -> c_int {
    unsafe {
        let mut ret: c_int = -1;

        /* HAVE_LINUX_COMPATIBLE_GETRANDOM */
        if global.getrandom_available != 0 {
            ret = 0;
        }

        sodium_memzero(
            stream_ptr() as *mut c_void,
            core::mem::size_of::<InternalRandom>(),
        );

        ret
    }
}

/* ------------------------------------------------------------------ */
/*  RDRAND is only used to mitigate prediction if a key is compromised */
/* ------------------------------------------------------------------ */

/// `static void randombytes_internal_random_xorhwrand(void)`
///
/// `HAVE_RDRAND` is undefined, so the body is empty.
#[inline(always)]
unsafe fn randombytes_internal_random_xorhwrand() {
    /* LCOV_EXCL_START */
    /* LCOV_EXCL_STOP */
}

/* ------------------------------------------------------------------ */
/*  XOR the key with another same-length secret                        */
/* ------------------------------------------------------------------ */

/// `static inline void randombytes_internal_random_xorkey(const unsigned char * const mix)`
#[inline]
unsafe fn randombytes_internal_random_xorkey(mix: *const u8) {
    let s = stream_ptr();
    let key: *mut u8 = (*s).key.as_mut_ptr();
    let mut i: usize;

    i = 0;
    while i < crypto_stream_chacha20_KEYBYTES {
        *key.add(i) ^= *mix.add(i);
        i += 1;
    }
}

/* ------------------------------------------------------------------ */
/*  Put `size` random bytes into `buf` and overwrite the key           */
/* ------------------------------------------------------------------ */

/// `static void randombytes_internal_random_buf(void * const buf, const size_t size)`
extern "C" fn randombytes_internal_random_buf(buf: *mut c_void, size: usize) {
    unsafe {
        let mut i: usize;
        let _ret: c_int;

        randombytes_internal_random_stir_if_needed();
        let s = stream_ptr();
        /* COMPILER_ASSERT(sizeof stream.nonce == crypto_stream_chacha20_NONCEBYTES) */
        const _: () = assert!(core::mem::size_of::<u64>() == crypto_stream_chacha20_NONCEBYTES);
        _ret = crypto_stream_chacha20(
            buf as *mut u8,
            size as c_ulonglong,
            core::ptr::addr_of_mut!((*s).nonce) as *const u8,
            (*s).key.as_ptr(),
        );
        /* assert(ret == 0);  -> no-op under NDEBUG */
        /* stream.key[i] ^= ((const unsigned char *) &size)[i], i < sizeof size */
        let size_bytes = size.to_ne_bytes();
        i = 0;
        while i < core::mem::size_of::<usize>() {
            (*s).key[i] ^= size_bytes[i];
            i += 1;
        }
        randombytes_internal_random_xorhwrand();
        (*s).nonce = (*s).nonce.wrapping_add(1);
        crypto_stream_chacha20_xor(
            (*s).key.as_mut_ptr(),
            (*s).key.as_ptr(),
            crypto_stream_chacha20_KEYBYTES as c_ulonglong,
            core::ptr::addr_of_mut!((*s).nonce) as *const u8,
            (*s).key.as_ptr(),
        );
    }
}

/* ------------------------------------------------------------------ */
/*  Pop a 32-bit value from the random pool                            */
/* ------------------------------------------------------------------ */

/// `static uint32_t randombytes_internal_random(void)`
extern "C" fn randombytes_internal_random() -> u32 {
    unsafe {
        let val: u32;
        let _ret: c_int;

        const RND32_LEN: usize = 16 * INTERNAL_RANDOM_BLOCK_SIZE;
        /* COMPILER_ASSERT(sizeof stream.rnd32 >= (sizeof stream.key) + (sizeof val)) */
        const _: () = assert!(RND32_LEN >= crypto_stream_chacha20_KEYBYTES + 4);
        /* COMPILER_ASSERT(((sizeof stream.rnd32) - (sizeof stream.key)) % sizeof val == 0) */
        const _: () = assert!((RND32_LEN - crypto_stream_chacha20_KEYBYTES) % 4 == 0);

        let s = stream_ptr();
        if (*s).rnd32_outleft == 0 {
            randombytes_internal_random_stir_if_needed();
            /* COMPILER_ASSERT(sizeof stream.nonce == crypto_stream_chacha20_NONCEBYTES) */
            const _: () =
                assert!(core::mem::size_of::<u64>() == crypto_stream_chacha20_NONCEBYTES);
            _ret = crypto_stream_chacha20(
                (*s).rnd32.as_mut_ptr(),
                RND32_LEN as c_ulonglong,
                core::ptr::addr_of_mut!((*s).nonce) as *const u8,
                (*s).key.as_ptr(),
            );
            /* assert(ret == 0);  -> no-op under NDEBUG */
            (*s).rnd32_outleft = RND32_LEN - crypto_stream_chacha20_KEYBYTES;
            randombytes_internal_random_xorhwrand();
            randombytes_internal_random_xorkey(
                (*s).rnd32.as_ptr().add((*s).rnd32_outleft),
            );
            core::ptr::write_bytes(
                (*s).rnd32.as_mut_ptr().add((*s).rnd32_outleft),
                0u8,
                crypto_stream_chacha20_KEYBYTES,
            );
            (*s).nonce = (*s).nonce.wrapping_add(1);
        }
        (*s).rnd32_outleft -= 4;
        val = core::ptr::read_unaligned(
            (*s).rnd32.as_ptr().add((*s).rnd32_outleft) as *const u32
        );
        core::ptr::write_bytes((*s).rnd32.as_mut_ptr().add((*s).rnd32_outleft), 0u8, 4);

        val
    }
}

/// `static const char *randombytes_internal_implementation_name(void)`
extern "C" fn randombytes_internal_implementation_name() -> *const c_char {
    c"internal".as_ptr()
}

/* ------------------------------------------------------------------ */
/*  exported data symbol                                              */
/* ------------------------------------------------------------------ */

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
