//! Shared OS-entropy plumbing used by both `randombytes_sysrandom` and
//! `randombytes_internal_random`.
//!
//! On Linux without `HAVE_GETRANDOM`, libsodium falls back to
//! `syscall(SYS_getrandom, ...)`, which is what we do here.  `BLOCK_ON_DEV_RANDOM`
//! is defined (Linux, `NO_BLOCKING_RANDOM_POLL` unset).
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_long, c_void};

use crate::common::{EAGAIN, EINTR, EIO, get_errno, set_errno, syscall};

#[cfg(target_arch = "x86_64")]
pub const SYS_GETRANDOM: c_long = 318;
#[cfg(target_arch = "x86")]
pub const SYS_GETRANDOM: c_long = 355;
#[cfg(target_arch = "aarch64")]
pub const SYS_GETRANDOM: c_long = 278;
#[cfg(target_arch = "arm")]
pub const SYS_GETRANDOM: c_long = 384;
#[cfg(not(any(
    target_arch = "x86_64",
    target_arch = "x86",
    target_arch = "aarch64",
    target_arch = "arm"
)))]
pub const SYS_GETRANDOM: c_long = 278;

pub const O_RDONLY: c_int = 0;
pub const F_GETFD: c_int = 1;
pub const F_SETFD: c_int = 2;
pub const FD_CLOEXEC: c_int = 1;
pub const POLLIN: i16 = 0x001;
pub const S_IFMT: u32 = 0o170000;
pub const S_IFCHR: u32 = 0o020000;

unsafe extern "C" {
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn close(fd: c_int) -> c_int;
    fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    fn poll(fds: *mut PollFd, nfds: u64, timeout: c_int) -> c_int;
    fn gettimeofday(tv: *mut TimeVal, tz: *mut c_void) -> c_int;
}

#[repr(C)]
pub struct PollFd {
    pub fd: c_int,
    pub events: i16,
    pub revents: i16,
}

#[repr(C)]
pub struct TimeVal {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

/// `sodium_hrtime()`
pub fn sodium_hrtime() -> u64 {
    let mut tv = TimeVal {
        tv_sec: 0,
        tv_usec: 0,
    };
    if unsafe { gettimeofday(&mut tv, core::ptr::null_mut()) } != 0 {
        crate::sodium::core::sodium_misuse();
    }
    (tv.tv_sec as u64) * 1_000_000 + (tv.tv_usec as u64)
}

/// `_randombytes_linux_getrandom()`
unsafe fn linux_getrandom_chunk(buf: *mut c_void, size: usize) -> c_int {
    let mut readnb: c_int;
    loop {
        readnb = unsafe { syscall(SYS_GETRANDOM, buf, size as c_int, 0 as c_int) } as c_int;
        if !(readnb < 0 && (get_errno() == EINTR || get_errno() == EAGAIN)) {
            break;
        }
    }
    ((readnb == size as c_int) as c_int) - 1
}

/// `randombytes_linux_getrandom()`
pub unsafe fn linux_getrandom(buf_: *mut c_void, mut size: usize) -> c_int {
    let mut buf = buf_ as *mut u8;
    let mut chunk_size: usize = 256;

    loop {
        if size < chunk_size {
            chunk_size = size;
        }
        if unsafe { linux_getrandom_chunk(buf as *mut c_void, chunk_size) } != 0 {
            return -1;
        }
        size -= chunk_size;
        buf = unsafe { buf.add(chunk_size) };
        if size == 0 {
            break;
        }
    }

    0
}

/// `randombytes_block_on_dev_random()`
pub fn block_on_dev_random() -> c_int {
    let fd = unsafe { open(b"/dev/random\0".as_ptr() as *const c_char, O_RDONLY) };
    if fd == -1 {
        return 0;
    }
    let mut pfd = PollFd {
        fd,
        events: POLLIN,
        revents: 0,
    };
    let mut pret;
    loop {
        pret = unsafe { poll(&mut pfd, 1, -1) };
        if !(pret < 0 && (get_errno() == EINTR || get_errno() == EAGAIN)) {
            break;
        }
    }
    if pret != 1 {
        unsafe { close(fd) };
        set_errno(EIO);
        return -1;
    }
    unsafe { close(fd) }
}

/// `randombytes_*_random_dev_open()`
///
/// `USE_BLOCKING_RANDOM` is not defined, so `/dev/urandom` is tried first.
pub fn random_dev_open() -> c_int {
    const DEVICES: [&[u8]; 2] = [b"/dev/urandom\0", b"/dev/random\0"];

    if block_on_dev_random() != 0 {
        return -1;
    }
    let mut idx = 0usize;
    while idx < DEVICES.len() {
        let fd = unsafe { open(DEVICES[idx].as_ptr() as *const c_char, O_RDONLY) };
        if fd != -1 {
            let mut st = [0u8; 256];
            if unsafe { fstat_raw(fd, st.as_mut_ptr()) } == 0 {
                // st_mode lives at offset 24 in the x86-64 Linux `struct stat`.
                let mode = u32::from_le_bytes([st[24], st[25], st[26], st[27]]);
                if (mode & S_IFMT) == S_IFCHR {
                    let flags = unsafe { fcntl(fd, F_GETFD) };
                    unsafe { fcntl(fd, F_SETFD, flags | FD_CLOEXEC) };
                    return fd;
                }
            }
            unsafe { close(fd) };
        } else if get_errno() == EINTR {
            continue;
        }
        idx += 1;
    }

    set_errno(EIO);
    -1
}

#[cfg(target_arch = "x86_64")]
const SYS_FSTAT: c_long = 5;
#[cfg(target_arch = "aarch64")]
const SYS_FSTAT: c_long = 80;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const SYS_FSTAT: c_long = 5;

unsafe fn fstat_raw(fd: c_int, buf: *mut u8) -> c_int {
    unsafe { syscall(SYS_FSTAT, fd, buf) as c_int }
}

/// `safe_read()`
pub unsafe fn safe_read(fd: c_int, buf_: *mut c_void, mut size: usize) -> isize {
    let mut buf = buf_ as *mut u8;
    let mut readnb: isize;

    loop {
        loop {
            readnb = unsafe { read(fd, buf as *mut c_void, size) };
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
        buf = unsafe { buf.add(readnb as usize) };
        if size == 0 {
            break;
        }
    }

    (buf as usize - buf_ as usize) as isize
}

pub fn close_fd(fd: c_int) -> c_int {
    unsafe { close(fd) }
}
