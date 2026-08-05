//! Translated from randombytes/randombytes.c, sysrandom, internal_random.
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_void};

extern "C" {
    fn sodium_misuse() -> !;
    fn sodium_runtime_has_rdrand() -> c_int;
    fn crypto_stream_chacha20(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
    fn crypto_stream_chacha20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_chacha20_ietf(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
}

pub const RANDOMBYTES_SEEDBYTES: usize = 32;

#[repr(C)]
pub struct RandombytesImplementation {
    pub implementation_name: Option<extern "C" fn() -> *const c_char>,
    pub random: Option<extern "C" fn() -> u32>,
    pub stir: Option<extern "C" fn()>,
    pub uniform: Option<extern "C" fn(u32) -> u32>,
    pub buf: Option<extern "C" fn(*mut c_void, usize)>,
    pub close: Option<extern "C" fn() -> c_int>,
}

unsafe impl Sync for RandombytesImplementation {}

static mut IMPLEMENTATION: *const RandombytesImplementation = core::ptr::null();

// ---------------- sysrandom implementation ----------------

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

const SSIZE_MAX: usize = usize::MAX / 2 - 1;

unsafe fn safe_read(fd: c_int, buf_: *mut c_void, mut size: usize) -> isize {
    let mut buf = buf_ as *mut u8;
    let mut readnb: isize;
    loop {
        loop {
            readnb = libc::read(fd, buf as *mut c_void, size);
            if readnb < 0 {
                let e = *libc::__errno_location();
                if e == libc::EINTR || e == libc::EAGAIN {
                    continue;
                }
            }
            break;
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
    buf as isize - buf_ as isize
}

unsafe fn linux_getrandom_chunk(buf: *mut c_void, size: usize) -> c_int {
    let mut readnb: libc::ssize_t;
    loop {
        readnb = libc::syscall(libc::SYS_getrandom, buf, size, 0) as libc::ssize_t;
        if readnb < 0 {
            let e = *libc::__errno_location();
            if e == libc::EINTR || e == libc::EAGAIN {
                continue;
            }
        }
        break;
    }
    ((readnb == size as libc::ssize_t) as c_int) - 1
}

unsafe fn linux_getrandom(buf_: *mut c_void, mut size: usize) -> c_int {
    let mut buf = buf_ as *mut u8;
    let mut chunk_size = 256usize;
    loop {
        if size < chunk_size {
            chunk_size = size;
        }
        if linux_getrandom_chunk(buf as *mut c_void, chunk_size) != 0 {
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

unsafe fn sysrandom_random_dev_open() -> c_int {
    let devices: [&[u8]; 2] = [b"/dev/urandom\0", b"/dev/random\0"];
    for dev in devices.iter() {
        let fd = libc::open(dev.as_ptr() as *const c_char, libc::O_RDONLY);
        if fd != -1 {
            let mut st: libc::stat = core::mem::zeroed();
            if libc::fstat(fd, &mut st) == 0 && (st.st_mode & libc::S_IFMT) == libc::S_IFCHR {
                let flags = libc::fcntl(fd, libc::F_GETFD);
                libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
                return fd;
            }
            libc::close(fd);
        } else if *libc::__errno_location() == libc::EINTR {
            continue;
        }
    }
    *libc::__errno_location() = libc::EIO;
    -1
}

unsafe fn sysrandom_init() {
    let errno_save = *libc::__errno_location();
    let mut fodder = [0u8; 16];
    if linux_getrandom(fodder.as_mut_ptr() as *mut c_void, 16) == 0 {
        SYS_STREAM.getrandom_available = 1;
        *libc::__errno_location() = errno_save;
        return;
    }
    SYS_STREAM.getrandom_available = 0;
    SYS_STREAM.random_data_source_fd = sysrandom_random_dev_open();
    if SYS_STREAM.random_data_source_fd == -1 {
        sodium_misuse();
    }
    *libc::__errno_location() = errno_save;
}

extern "C" fn sysrandom_stir() {
    unsafe {
        if SYS_STREAM.initialized == 0 {
            sysrandom_init();
            SYS_STREAM.initialized = 1;
        }
    }
}

unsafe fn sysrandom_stir_if_needed() {
    if SYS_STREAM.initialized == 0 {
        sysrandom_stir();
    }
}

extern "C" fn sysrandom_close() -> c_int {
    unsafe {
        let mut ret = -1;
        if SYS_STREAM.random_data_source_fd != -1
            && libc::close(SYS_STREAM.random_data_source_fd) == 0
        {
            SYS_STREAM.random_data_source_fd = -1;
            SYS_STREAM.initialized = 0;
            ret = 0;
        }
        if SYS_STREAM.getrandom_available != 0 {
            ret = 0;
        }
        ret
    }
}

extern "C" fn sysrandom_buf(buf: *mut c_void, size: usize) {
    unsafe {
        sysrandom_stir_if_needed();
        if SYS_STREAM.getrandom_available != 0 {
            if linux_getrandom(buf, size) != 0 {
                sodium_misuse();
            }
            return;
        }
        if SYS_STREAM.random_data_source_fd == -1
            || safe_read(SYS_STREAM.random_data_source_fd, buf, size) != size as isize
        {
            sodium_misuse();
        }
    }
}

extern "C" fn sysrandom() -> u32 {
    let mut r: u32 = 0;
    sysrandom_buf(&mut r as *mut u32 as *mut c_void, 4);
    r
}

extern "C" fn sysrandom_implementation_name() -> *const c_char {
    b"sysrandom\0".as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub static randombytes_sysrandom_implementation: RandombytesImplementation =
    RandombytesImplementation {
        implementation_name: Some(sysrandom_implementation_name),
        random: Some(sysrandom),
        stir: Some(sysrandom_stir),
        uniform: None,
        buf: Some(sysrandom_buf),
        close: Some(sysrandom_close),
    };

// ---------------- internal (chacha20-based) implementation ----------------

const CHACHA20_KEYBYTES: usize = 32;
const INTERNAL_RANDOM_BLOCK_SIZE: usize = 32; // crypto_core_hchacha20_OUTPUTBYTES

struct InternalRandomGlobal {
    initialized: c_int,
    random_data_source_fd: c_int,
    getentropy_available: c_int,
    getrandom_available: c_int,
    rdrand_available: c_int,
    pid: libc::pid_t,
}

struct InternalRandom {
    initialized: c_int,
    rnd32_outleft: usize,
    key: [u8; CHACHA20_KEYBYTES],
    rnd32: [u8; 16 * INTERNAL_RANDOM_BLOCK_SIZE],
    nonce: u64,
}

static mut INT_GLOBAL: InternalRandomGlobal = InternalRandomGlobal {
    initialized: 0,
    random_data_source_fd: -1,
    getentropy_available: 0,
    getrandom_available: 0,
    rdrand_available: 0,
    pid: 0,
};

static mut INT_STREAM: InternalRandom = InternalRandom {
    initialized: 0,
    rnd32_outleft: 0,
    key: [0u8; CHACHA20_KEYBYTES],
    rnd32: [0u8; 16 * INTERNAL_RANDOM_BLOCK_SIZE],
    nonce: 0,
};

unsafe fn sodium_hrtime() -> u64 {
    let mut tv: libc::timeval = core::mem::zeroed();
    if libc::gettimeofday(&mut tv, core::ptr::null_mut()) != 0 {
        sodium_misuse();
    }
    (tv.tv_sec as u64) * 1000000 + (tv.tv_usec as u64)
}

unsafe fn internal_random_init() {
    let errno_save = *libc::__errno_location();
    INT_GLOBAL.rdrand_available = sodium_runtime_has_rdrand();
    INT_GLOBAL.getentropy_available = 0;
    INT_GLOBAL.getrandom_available = 0;
    let mut fodder = [0u8; 16];
    if linux_getrandom(fodder.as_mut_ptr() as *mut c_void, 16) == 0 {
        INT_GLOBAL.getrandom_available = 1;
        *libc::__errno_location() = errno_save;
        return;
    }
    INT_GLOBAL.random_data_source_fd = sysrandom_random_dev_open();
    if INT_GLOBAL.random_data_source_fd == -1 {
        sodium_misuse();
    }
    *libc::__errno_location() = errno_save;
}

extern "C" fn internal_random_stir() {
    unsafe {
        INT_STREAM.nonce = sodium_hrtime();
        INT_STREAM.rnd32 = [0u8; 16 * INTERNAL_RANDOM_BLOCK_SIZE];
        INT_STREAM.rnd32_outleft = 0;
        if INT_GLOBAL.initialized == 0 {
            internal_random_init();
            INT_GLOBAL.initialized = 1;
        }
        INT_GLOBAL.pid = libc::getpid();
        if INT_GLOBAL.getrandom_available != 0 {
            if linux_getrandom(INT_STREAM.key.as_mut_ptr() as *mut c_void, CHACHA20_KEYBYTES) != 0 {
                sodium_misuse();
            }
        } else if INT_GLOBAL.random_data_source_fd == -1
            || safe_read(
                INT_GLOBAL.random_data_source_fd,
                INT_STREAM.key.as_mut_ptr() as *mut c_void,
                CHACHA20_KEYBYTES,
            ) != CHACHA20_KEYBYTES as isize
        {
            sodium_misuse();
        }
        INT_STREAM.initialized = 1;
    }
}

unsafe fn internal_random_stir_if_needed() {
    if INT_STREAM.initialized == 0 {
        internal_random_stir();
    } else if INT_GLOBAL.pid != libc::getpid() {
        sodium_misuse();
    }
}

extern "C" fn internal_random_close() -> c_int {
    unsafe {
        let mut ret = -1;
        if INT_GLOBAL.getrandom_available != 0 {
            ret = 0;
        } else if INT_GLOBAL.random_data_source_fd != -1
            && libc::close(INT_GLOBAL.random_data_source_fd) == 0
        {
            INT_GLOBAL.random_data_source_fd = -1;
            INT_GLOBAL.initialized = 0;
            INT_GLOBAL.pid = 0;
            ret = 0;
        }
        sodium_memzero_local(
            core::ptr::addr_of_mut!(INT_STREAM) as *mut c_void,
            core::mem::size_of::<InternalRandom>(),
        );
        ret
    }
}

unsafe fn sodium_memzero_local(pnt: *mut c_void, len: usize) {
    let p = pnt as *mut u8;
    for i in 0..len {
        core::ptr::write_volatile(p.add(i), 0);
    }
}

unsafe fn internal_random_xorhwrand() {
    // No HAVE_RDRAND -> no-op
}

unsafe fn internal_random_xorkey(mix: *const u8) {
    for i in 0..CHACHA20_KEYBYTES {
        INT_STREAM.key[i] ^= *mix.add(i);
    }
}

extern "C" fn internal_random_buf(buf: *mut c_void, size: usize) {
    unsafe {
        internal_random_stir_if_needed();
        let ret = crypto_stream_chacha20(
            buf as *mut u8,
            size as u64,
            core::ptr::addr_of_mut!(INT_STREAM.nonce) as *const u8,
            INT_STREAM.key.as_ptr(),
        );
        debug_assert!(ret == 0);
        let size_bytes = size.to_ne_bytes();
        for i in 0..core::mem::size_of::<usize>() {
            INT_STREAM.key[i] ^= size_bytes[i];
        }
        internal_random_xorhwrand();
        INT_STREAM.nonce = INT_STREAM.nonce.wrapping_add(1);
        crypto_stream_chacha20_xor(
            INT_STREAM.key.as_mut_ptr(),
            INT_STREAM.key.as_ptr(),
            CHACHA20_KEYBYTES as u64,
            core::ptr::addr_of_mut!(INT_STREAM.nonce) as *const u8,
            INT_STREAM.key.as_ptr(),
        );
    }
}

extern "C" fn internal_random() -> u32 {
    unsafe {
        let val_size = 4usize;
        if INT_STREAM.rnd32_outleft == 0 {
            internal_random_stir_if_needed();
            let ret = crypto_stream_chacha20(
                INT_STREAM.rnd32.as_mut_ptr(),
                INT_STREAM.rnd32.len() as u64,
                core::ptr::addr_of_mut!(INT_STREAM.nonce) as *const u8,
                INT_STREAM.key.as_ptr(),
            );
            debug_assert!(ret == 0);
            INT_STREAM.rnd32_outleft = INT_STREAM.rnd32.len() - CHACHA20_KEYBYTES;
            internal_random_xorhwrand();
            let off = INT_STREAM.rnd32_outleft;
            let mix = INT_STREAM.rnd32.as_ptr().add(off);
            internal_random_xorkey(mix);
            core::ptr::write_bytes(INT_STREAM.rnd32.as_mut_ptr().add(off), 0, CHACHA20_KEYBYTES);
            INT_STREAM.nonce = INT_STREAM.nonce.wrapping_add(1);
        }
        INT_STREAM.rnd32_outleft -= val_size;
        let off = INT_STREAM.rnd32_outleft;
        let mut val: u32 = 0;
        core::ptr::copy_nonoverlapping(
            INT_STREAM.rnd32.as_ptr().add(off),
            &mut val as *mut u32 as *mut u8,
            val_size,
        );
        core::ptr::write_bytes(INT_STREAM.rnd32.as_mut_ptr().add(off), 0, val_size);
        val
    }
}

extern "C" fn internal_implementation_name() -> *const c_char {
    b"internal\0".as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub static randombytes_internal_implementation: RandombytesImplementation =
    RandombytesImplementation {
        implementation_name: Some(internal_implementation_name),
        random: Some(internal_random),
        stir: Some(internal_random_stir),
        uniform: None,
        buf: Some(internal_random_buf),
        close: Some(internal_random_close),
    };

// ---------------- public API ----------------

unsafe fn randombytes_init_if_needed() {
    if IMPLEMENTATION.is_null() {
        IMPLEMENTATION = &randombytes_sysrandom_implementation;
        randombytes_stir();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_set_implementation(
    impl_: *const RandombytesImplementation,
) -> c_int {
    IMPLEMENTATION = impl_;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_implementation_name() -> *const c_char {
    randombytes_init_if_needed();
    ((*IMPLEMENTATION).implementation_name.unwrap())()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_random() -> u32 {
    randombytes_init_if_needed();
    ((*IMPLEMENTATION).random.unwrap())()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_stir() {
    randombytes_init_if_needed();
    if let Some(stir) = (*IMPLEMENTATION).stir {
        stir();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_uniform(upper_bound: u32) -> u32 {
    randombytes_init_if_needed();
    if let Some(uniform) = (*IMPLEMENTATION).uniform {
        return uniform(upper_bound);
    }
    if upper_bound < 2 {
        return 0;
    }
    let min = (1u32.wrapping_add(!upper_bound)) % upper_bound;
    let mut r;
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
        ((*IMPLEMENTATION).buf.unwrap())(buf, size);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_buf_deterministic(
    buf: *mut c_void,
    size: usize,
    seed: *const u8,
) {
    static NONCE: [u8; 12] = [
        b'L', b'i', b'b', b's', b'o', b'd', b'i', b'u', b'm', b'D', b'R', b'G',
    ];
    crypto_stream_chacha20_ietf(
        buf as *mut u8,
        size as u64,
        NONCE.as_ptr(),
        seed,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_seedbytes() -> usize {
    RANDOMBYTES_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_close() -> c_int {
    if !IMPLEMENTATION.is_null() {
        if let Some(close) = (*IMPLEMENTATION).close {
            return close();
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(buf: *mut u8, buf_len: u64) {
    randombytes_buf(buf as *mut c_void, buf_len as usize);
}
