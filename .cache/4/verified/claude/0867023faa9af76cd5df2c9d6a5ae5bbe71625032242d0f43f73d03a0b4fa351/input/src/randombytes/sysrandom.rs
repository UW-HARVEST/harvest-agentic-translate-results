//! `randombytes/sysrandom/randombytes_sysrandom.c`

use core::ffi::{c_char, c_int, c_void};

use super::RandombytesImplementation;
use super::os;
use crate::common::{get_errno, set_errno};
use crate::sodium::core::sodium_misuse;

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

#[inline]
fn st() -> &'static mut SysRandom {
    unsafe { &mut *(&raw mut STREAM) }
}

fn randombytes_sysrandom_init() {
    let errno_save = get_errno();

    {
        let mut fodder = [0u8; 16];
        if unsafe { os::linux_getrandom(fodder.as_mut_ptr() as *mut c_void, 16) } == 0 {
            st().getrandom_available = 1;
            set_errno(errno_save);
            return;
        }
        st().getrandom_available = 0;
    }

    st().random_data_source_fd = os::random_dev_open();
    if st().random_data_source_fd == -1 {
        sodium_misuse();
    }
    set_errno(errno_save);
}

extern "C" fn randombytes_sysrandom_stir() {
    if st().initialized == 0 {
        randombytes_sysrandom_init();
        st().initialized = 1;
    }
}

fn randombytes_sysrandom_stir_if_needed() {
    if st().initialized == 0 {
        randombytes_sysrandom_stir();
    }
}

extern "C" fn randombytes_sysrandom_close() -> c_int {
    let mut ret: c_int = -1;

    if st().random_data_source_fd != -1 && os::close_fd(st().random_data_source_fd) == 0 {
        st().random_data_source_fd = -1;
        st().initialized = 0;
        ret = 0;
    }
    if st().getrandom_available != 0 {
        ret = 0;
    }
    ret
}

unsafe extern "C" fn randombytes_sysrandom_buf(buf: *mut c_void, size: usize) {
    randombytes_sysrandom_stir_if_needed();
    if st().getrandom_available != 0 {
        if unsafe { os::linux_getrandom(buf, size) } != 0 {
            sodium_misuse();
        }
        return;
    }
    if st().random_data_source_fd == -1
        || unsafe { os::safe_read(st().random_data_source_fd, buf, size) } != size as isize
    {
        sodium_misuse();
    }
}

extern "C" fn randombytes_sysrandom() -> u32 {
    let mut r: u32 = 0;
    unsafe { randombytes_sysrandom_buf(&raw mut r as *mut c_void, 4) };
    r
}

extern "C" fn randombytes_sysrandom_implementation_name() -> *const c_char {
    b"sysrandom\0".as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub static mut randombytes_sysrandom_implementation: RandombytesImplementation =
    RandombytesImplementation {
        implementation_name: Some(randombytes_sysrandom_implementation_name),
        random: Some(randombytes_sysrandom),
        stir: Some(randombytes_sysrandom_stir),
        uniform: None,
        buf: Some(randombytes_sysrandom_buf),
        close: Some(randombytes_sysrandom_close),
    };
