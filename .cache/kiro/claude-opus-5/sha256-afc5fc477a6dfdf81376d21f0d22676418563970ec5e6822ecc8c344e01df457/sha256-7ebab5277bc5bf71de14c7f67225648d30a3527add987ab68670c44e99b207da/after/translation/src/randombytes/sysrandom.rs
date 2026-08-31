//! Translation of `libsodium/randombytes/sysrandom/randombytes_sysrandom.c`
//!
//! On Linux the reference build selects the `getrandom(2)` syscall path
//! (`HAVE_LINUX_COMPATIBLE_GETRANDOM` via `SYS_getrandom`).

use core::ffi::{c_char, c_int, c_long, c_void};

use super::randombytes_implementation;
use crate::plat::{get_errno, set_errno, EAGAIN, EINTR, EIO};
use crate::sodium::core::sodium_misuse;

extern "C" {
    fn syscall(num: c_long, ...) -> c_long;
    fn open(path: *const c_char, oflag: c_int, ...) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn close(fd: c_int) -> c_int;
}

/// `__NR_getrandom` on x86_64.
const SYS_GETRANDOM: c_long = 318;
const O_RDONLY: c_int = 0;

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

unsafe fn randombytes_sysrandom_random_dev_open() -> c_int {
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

unsafe fn randombytes_sysrandom_init() {
    let errno_save = get_errno();

    {
        let mut fodder = [0u8; 16];
        if randombytes_linux_getrandom(fodder.as_mut_ptr() as *mut c_void, 16) == 0 {
            STREAM.getrandom_available = 1;
            set_errno(errno_save);
            return;
        }
        STREAM.getrandom_available = 0;
    }

    STREAM.random_data_source_fd = randombytes_sysrandom_random_dev_open();
    if STREAM.random_data_source_fd == -1 {
        sodium_misuse();
    }
    set_errno(errno_save);
}

extern "C" fn randombytes_sysrandom_stir() {
    unsafe {
        if STREAM.initialized == 0 {
            randombytes_sysrandom_init();
            STREAM.initialized = 1;
        }
    }
}

unsafe fn randombytes_sysrandom_stir_if_needed() {
    if STREAM.initialized == 0 {
        randombytes_sysrandom_stir();
    }
}

extern "C" fn randombytes_sysrandom_close() -> c_int {
    let mut ret: c_int = -1;

    unsafe {
        if STREAM.random_data_source_fd != -1 && close(STREAM.random_data_source_fd) == 0 {
            STREAM.random_data_source_fd = -1;
            STREAM.initialized = 0;
            ret = 0;
        }
        if STREAM.getrandom_available != 0 {
            ret = 0;
        }
    }
    ret
}

unsafe extern "C" fn randombytes_sysrandom_buf(buf: *mut c_void, size: usize) {
    randombytes_sysrandom_stir_if_needed();
    if STREAM.getrandom_available != 0 {
        if randombytes_linux_getrandom(buf, size) != 0 {
            sodium_misuse();
        }
        return;
    }
    if STREAM.random_data_source_fd == -1
        || safe_read(STREAM.random_data_source_fd, buf, size) != size as isize
    {
        sodium_misuse();
    }
}

extern "C" fn randombytes_sysrandom() -> u32 {
    let mut r: u32 = 0;
    unsafe {
        randombytes_sysrandom_buf(&mut r as *mut u32 as *mut c_void, 4);
    }
    r
}

extern "C" fn randombytes_sysrandom_implementation_name() -> *const c_char {
    b"sysrandom\0".as_ptr() as *const c_char
}

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
