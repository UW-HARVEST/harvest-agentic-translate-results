//! Translation of `randombytes/randombytes.c`,
//! `randombytes/sysrandom/randombytes_sysrandom.c` and
//! `randombytes/internal/randombytes_internal_random.c`.

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::common::{memcpy, memset};
use crate::sodium_core::sodium_misuse;

pub const randombytes_SEEDBYTES: usize = 32;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct randombytes_implementation {
    pub implementation_name: Option<extern "C" fn() -> *const c_char>,
    pub random: Option<extern "C" fn() -> u32>,
    pub stir: Option<extern "C" fn()>,
    pub uniform: Option<extern "C" fn(u32) -> u32>,
    pub buf: Option<unsafe extern "C" fn(*mut c_void, usize)>,
    pub close: Option<extern "C" fn() -> c_int>,
}

unsafe impl Sync for randombytes_implementation {}

static mut IMPLEMENTATION: *const randombytes_implementation = ptr::null();

fn randombytes_init_if_needed() {
    unsafe {
        if (*(&raw const IMPLEMENTATION)).is_null() {
            *(&raw mut IMPLEMENTATION) =
                (&raw const randombytes_sysrandom_implementation) as *const _;
            randombytes_stir();
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_set_implementation(
    impl_: *const randombytes_implementation,
) -> c_int {
    *(&raw mut IMPLEMENTATION) = impl_;
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_implementation_name() -> *const c_char {
    randombytes_init_if_needed();
    unsafe { ((*(*(&raw const IMPLEMENTATION))).implementation_name.unwrap())() }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_random() -> u32 {
    randombytes_init_if_needed();
    unsafe { ((*(*(&raw const IMPLEMENTATION))).random.unwrap())() }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_stir() {
    randombytes_init_if_needed();
    unsafe {
        if let Some(stir) = (*(*(&raw const IMPLEMENTATION))).stir {
            stir();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_uniform(upper_bound: u32) -> u32 {
    let min: u32;
    let mut r: u32;

    randombytes_init_if_needed();
    unsafe {
        if let Some(uniform) = (*(*(&raw const IMPLEMENTATION))).uniform {
            return uniform(upper_bound);
        }
    }
    if upper_bound < 2 {
        return 0;
    }
    min = (1u32.wrapping_add(!upper_bound)) % upper_bound;
    loop {
        r = randombytes_random();
        if r >= min {
            break;
        }
    }

    r % upper_bound
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_buf(buf: *mut c_void, size: usize) {
    randombytes_init_if_needed();
    if size > 0 {
        ((*(*(&raw const IMPLEMENTATION))).buf.unwrap())(buf, size);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_buf_deterministic(
    buf: *mut c_void,
    size: usize,
    seed: *const u8,
) {
    static NONCE: [u8; 12] = [b'L', b'i', b'b', b's', b'o', b'd', b'i', b'u', b'm', b'D', b'R', b'G'];

    if size > 0x4000000000u64 as usize {
        sodium_misuse();
    }
    crate::crypto_stream::chacha20::crypto_stream_chacha20_ietf(
        buf as *mut u8,
        size as u64,
        NONCE.as_ptr(),
        seed,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_seedbytes() -> usize {
    randombytes_SEEDBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_close() -> c_int {
    unsafe {
        let imp = *(&raw const IMPLEMENTATION);
        if !imp.is_null() {
            if let Some(close) = (*imp).close {
                return close();
            }
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(buf: *mut u8, buf_len: u64) {
    randombytes_buf(buf as *mut c_void, buf_len as usize);
}

/* ------------------------------------------------------------------ */
/* randombytes/sysrandom/randombytes_sysrandom.c                       */
/* ------------------------------------------------------------------ */

struct SysRandom {
    random_data_source_fd: c_int,
    initialized: c_int,
    getrandom_available: c_int,
}

static mut SYS_STREAM: SysRandom = SysRandom {
    random_data_source_fd: -1,
    initialized: 0,
    getrandom_available: 0,
};

unsafe fn safe_read(fd: c_int, buf_: *mut c_void, mut size: usize) -> isize {
    let mut buf = buf_ as *mut u8;
    let mut readnb: isize;

    loop {
        loop {
            readnb = libc::read(fd, buf as *mut c_void, size);
            if !(readnb < 0
                && (crate::get_errno() == libc::EINTR || crate::get_errno() == libc::EAGAIN))
            {
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

unsafe fn randombytes_block_on_dev_random() -> c_int {
    let mut pfd: libc::pollfd = core::mem::zeroed();
    let fd: c_int;
    let mut pret: c_int;

    fd = libc::open(b"/dev/random\0".as_ptr() as *const c_char, libc::O_RDONLY);
    if fd == -1 {
        return 0;
    }
    pfd.fd = fd;
    pfd.events = libc::POLLIN;
    pfd.revents = 0;
    loop {
        pret = libc::poll(&mut pfd, 1, -1);
        if !(pret < 0
            && (crate::get_errno() == libc::EINTR || crate::get_errno() == libc::EAGAIN))
        {
            break;
        }
    }
    if pret != 1 {
        libc::close(fd);
        crate::set_errno(libc::EIO);
        return -1;
    }
    libc::close(fd)
}

unsafe fn random_dev_open() -> c_int {
    let mut st: libc::stat = core::mem::zeroed();
    const DEVICES: [Option<&[u8]>; 3] =
        [Some(b"/dev/urandom\0"), Some(b"/dev/random\0"), None];
    let mut idx = 0usize;
    let fd: c_int;

    if randombytes_block_on_dev_random() != 0 {
        return -1;
    }
    loop {
        let fd = libc::open(DEVICES[idx].unwrap().as_ptr() as *const c_char, libc::O_RDONLY);
        if fd != -1 {
            if libc::fstat(fd, &mut st) == 0 && (st.st_mode & libc::S_IFMT) == libc::S_IFCHR {
                libc::fcntl(
                    fd,
                    libc::F_SETFD,
                    libc::fcntl(fd, libc::F_GETFD) | libc::FD_CLOEXEC,
                );
                return fd;
            }
            libc::close(fd);
        } else if crate::get_errno() == libc::EINTR {
            continue;
        }
        idx += 1;
        if DEVICES[idx].is_none() {
            break;
        }
    }
    let _ = fd;

    crate::set_errno(libc::EIO);
    -1
}

unsafe fn getrandom_syscall(buf: *mut c_void, size: usize, flags: c_uint_alias) -> isize {
    libc::syscall(libc::SYS_getrandom, buf, size as c_int, flags) as isize
}

type c_uint_alias = c_int;

unsafe fn _linux_getrandom(buf: *mut c_void, size: usize) -> c_int {
    let mut readnb: c_int;

    loop {
        readnb = getrandom_syscall(buf, size, 0) as c_int;
        if !(readnb < 0
            && (crate::get_errno() == libc::EINTR || crate::get_errno() == libc::EAGAIN))
        {
            break;
        }
    }

    ((readnb == size as c_int) as c_int) - 1
}

unsafe fn linux_getrandom(buf_: *mut c_void, mut size: usize) -> c_int {
    let mut buf = buf_ as *mut u8;
    let mut chunk_size: usize = 256;

    loop {
        if size < chunk_size {
            chunk_size = size;
        }
        if _linux_getrandom(buf as *mut c_void, chunk_size) != 0 {
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
    let errno_save = crate::get_errno();

    {
        let mut fodder: [u8; 16] = [0; 16];

        if linux_getrandom(fodder.as_mut_ptr() as *mut c_void, 16) == 0 {
            (*(&raw mut SYS_STREAM)).getrandom_available = 1;
            crate::set_errno(errno_save);
            return;
        }
        (*(&raw mut SYS_STREAM)).getrandom_available = 0;
    }

    (*(&raw mut SYS_STREAM)).random_data_source_fd = random_dev_open();
    if (*(&raw const SYS_STREAM)).random_data_source_fd == -1 {
        sodium_misuse();
    }
    crate::set_errno(errno_save);
}

extern "C" fn randombytes_sysrandom_stir() {
    unsafe {
        if (*(&raw const SYS_STREAM)).initialized == 0 {
            randombytes_sysrandom_init();
            (*(&raw mut SYS_STREAM)).initialized = 1;
        }
    }
}

unsafe fn randombytes_sysrandom_stir_if_needed() {
    if (*(&raw const SYS_STREAM)).initialized == 0 {
        randombytes_sysrandom_stir();
    }
}

extern "C" fn randombytes_sysrandom_close() -> c_int {
    let mut ret: c_int = -1;

    unsafe {
        if (*(&raw const SYS_STREAM)).random_data_source_fd != -1
            && libc::close((*(&raw const SYS_STREAM)).random_data_source_fd) == 0
        {
            (*(&raw mut SYS_STREAM)).random_data_source_fd = -1;
            (*(&raw mut SYS_STREAM)).initialized = 0;
            ret = 0;
        }
        if (*(&raw const SYS_STREAM)).getrandom_available != 0 {
            ret = 0;
        }
    }
    ret
}

unsafe extern "C" fn randombytes_sysrandom_buf(buf: *mut c_void, size: usize) {
    randombytes_sysrandom_stir_if_needed();
    if (*(&raw const SYS_STREAM)).getrandom_available != 0 {
        if linux_getrandom(buf, size) != 0 {
            sodium_misuse();
        }
        return;
    }
    if (*(&raw const SYS_STREAM)).random_data_source_fd == -1
        || safe_read((*(&raw const SYS_STREAM)).random_data_source_fd, buf, size)
            != size as isize
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

/* ------------------------------------------------------------------ */
/* randombytes/internal/randombytes_internal_random.c                  */
/* ------------------------------------------------------------------ */

const INTERNAL_RANDOM_BLOCK_SIZE: usize = 32; /* crypto_core_hchacha20_OUTPUTBYTES */
const CHACHA20_KEYBYTES: usize = 32;

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
    key: [u8; CHACHA20_KEYBYTES],
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

// `TLS` expands to nothing because the reference build compiles with
// `-std=c99` (`__STDC_VERSION__ == 199901L`), so this is a plain static.
static mut INT_STREAM: InternalRandom = InternalRandom {
    initialized: 0,
    rnd32_outleft: 0,
    key: [0; CHACHA20_KEYBYTES],
    rnd32: [0; 16 * INTERNAL_RANDOM_BLOCK_SIZE],
    nonce: 0,
};

unsafe fn sodium_hrtime() -> u64 {
    let mut tv: libc::timeval = core::mem::zeroed();

    if libc::gettimeofday(&mut tv, ptr::null_mut()) != 0 {
        sodium_misuse();
    }
    (tv.tv_sec as u64) * 1000000u64 + (tv.tv_usec as u64)
}

unsafe fn randombytes_internal_random_init() {
    let errno_save = crate::get_errno();

    (*(&raw mut GLOBAL)).rdrand_available =
        crate::sodium_runtime::sodium_runtime_has_rdrand();
    (*(&raw mut GLOBAL)).getentropy_available = 0;
    (*(&raw mut GLOBAL)).getrandom_available = 0;

    {
        let mut fodder: [u8; 16] = [0; 16];

        if linux_getrandom(fodder.as_mut_ptr() as *mut c_void, 16) == 0 {
            (*(&raw mut GLOBAL)).getrandom_available = 1;
            crate::set_errno(errno_save);
            return;
        }
    }
    (*(&raw mut GLOBAL)).random_data_source_fd = random_dev_open();
    if (*(&raw const GLOBAL)).random_data_source_fd == -1 {
        sodium_misuse();
    }
    crate::set_errno(errno_save);
}

extern "C" fn randombytes_internal_random_stir() {
    unsafe {
        let s = &raw mut INT_STREAM;
        (*s).nonce = sodium_hrtime();
        memset((*s).rnd32.as_mut_ptr(), 0, 16 * INTERNAL_RANDOM_BLOCK_SIZE);
        (*s).rnd32_outleft = 0;
        if (*(&raw const GLOBAL)).initialized == 0 {
            randombytes_internal_random_init();
            (*(&raw mut GLOBAL)).initialized = 1;
        }

        if (*(&raw const GLOBAL)).getrandom_available != 0 {
            if linux_getrandom((*s).key.as_mut_ptr() as *mut c_void, CHACHA20_KEYBYTES) != 0 {
                sodium_misuse();
            }
        }

        (*s).initialized = 1;
    }
}

unsafe fn randombytes_internal_random_stir_if_needed() {
    if (*(&raw const INT_STREAM)).initialized == 0 {
        randombytes_internal_random_stir();
    }
}

extern "C" fn randombytes_internal_random_close() -> c_int {
    let mut ret: c_int = -1;

    unsafe {
        if (*(&raw const GLOBAL)).getrandom_available != 0 {
            ret = 0;
        }

        crate::sodium_utils::sodium_memzero(
            (&raw mut INT_STREAM) as *mut c_void,
            core::mem::size_of::<InternalRandom>(),
        );
    }

    ret
}

/// `randombytes_internal_random_xorhwrand()`: `HAVE_RDRAND` is undefined, so
/// the body is empty.
fn randombytes_internal_random_xorhwrand() {}

unsafe fn randombytes_internal_random_xorkey(mix: *const u8) {
    let key = (*(&raw mut INT_STREAM)).key.as_mut_ptr();

    for i in 0..CHACHA20_KEYBYTES {
        *key.add(i) ^= *mix.add(i);
    }
}

unsafe extern "C" fn randombytes_internal_random_buf(buf: *mut c_void, size: usize) {
    randombytes_internal_random_stir_if_needed();
    let s = &raw mut INT_STREAM;
    crate::crypto_stream::chacha20::crypto_stream_chacha20(
        buf as *mut u8,
        size as u64,
        (&raw mut (*s).nonce) as *const u8,
        (*s).key.as_ptr(),
    );
    let size_bytes = (&size) as *const usize as *const u8;
    for i in 0..core::mem::size_of::<usize>() {
        (*s).key[i] ^= *size_bytes.add(i);
    }
    randombytes_internal_random_xorhwrand();
    (*s).nonce = (*s).nonce.wrapping_add(1);
    crate::crypto_stream::chacha20::crypto_stream_chacha20_xor(
        (*s).key.as_mut_ptr(),
        (*s).key.as_ptr(),
        CHACHA20_KEYBYTES as u64,
        (&raw mut (*s).nonce) as *const u8,
        (*s).key.as_ptr(),
    );
}

extern "C" fn randombytes_internal_random() -> u32 {
    let mut val: u32 = 0;

    unsafe {
        let s = &raw mut INT_STREAM;
        if (*(&raw const INT_STREAM)).rnd32_outleft == 0 {
            randombytes_internal_random_stir_if_needed();
            crate::crypto_stream::chacha20::crypto_stream_chacha20(
                (*s).rnd32.as_mut_ptr(),
                (16 * INTERNAL_RANDOM_BLOCK_SIZE) as u64,
                (&raw mut (*s).nonce) as *const u8,
                (*s).key.as_ptr(),
            );
            (*s).rnd32_outleft = (16 * INTERNAL_RANDOM_BLOCK_SIZE) - CHACHA20_KEYBYTES;
            randombytes_internal_random_xorhwrand();
            let off = (*s).rnd32_outleft;
            randombytes_internal_random_xorkey((*s).rnd32.as_ptr().add(off));
            memset((*s).rnd32.as_mut_ptr().add(off), 0, CHACHA20_KEYBYTES);
            (*s).nonce = (*s).nonce.wrapping_add(1);
        }
        (*s).rnd32_outleft -= 4;
        let off = (*s).rnd32_outleft;
        memcpy(
            (&mut val) as *mut u32 as *mut u8,
            (*s).rnd32.as_ptr().add(off),
            4,
        );
        memset((*s).rnd32.as_mut_ptr().add(off), 0, 4);
    }

    val
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
