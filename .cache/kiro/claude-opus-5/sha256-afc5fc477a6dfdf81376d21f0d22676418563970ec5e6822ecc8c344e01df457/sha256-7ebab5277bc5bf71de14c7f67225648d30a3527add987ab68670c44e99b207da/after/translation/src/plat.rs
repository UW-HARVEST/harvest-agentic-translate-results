//! Small platform helpers shared by the translated modules.

use core::ffi::c_int;

extern "C" {
    fn __errno_location() -> *mut c_int;
}

pub const EPERM: c_int = 1;
pub const ENOENT: c_int = 2;
pub const EINTR: c_int = 4;
pub const EIO: c_int = 5;
pub const EAGAIN: c_int = 11;
pub const ENOMEM: c_int = 12;
pub const EINVAL: c_int = 22;
pub const ERANGE: c_int = 34;
pub const ENOSYS: c_int = 38;

#[inline]
pub fn set_errno(v: c_int) {
    unsafe {
        *__errno_location() = v;
    }
}

#[inline]
pub fn get_errno() -> c_int {
    unsafe { *__errno_location() }
}

/// `strchr(s, c) != NULL` for a NUL-terminated C string.
///
/// Note: C's `strchr` also matches the terminating NUL byte.
#[inline]
pub unsafe fn strchr_found(s: *const core::ffi::c_char, c: u8) -> bool {
    let mut p = s as *const u8;
    loop {
        let v = *p;
        if v == c {
            return true;
        }
        if v == 0 {
            return false;
        }
        p = p.add(1);
    }
}
