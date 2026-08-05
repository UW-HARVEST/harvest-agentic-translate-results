//! Translated from sodium/utils.c
//! Build defines no HAVE_ALIGNED_MALLOC / HAVE_MLOCK / HAVE_MPROTECT / HAVE_WEAK_SYMBOLS,
//! so the simple/portable paths are used.
#![allow(dead_code)]

use core::ffi::{c_int, c_void};

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_misuse() -> !;
}

const CANARY_SIZE: usize = 16;
const GARBAGE_VALUE: i32 = 0xdb;

static mut PAGE_SIZE: usize = 0x10000;
static mut CANARY: [u8; CANARY_SIZE] = [0u8; CANARY_SIZE];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_memzero(pnt: *mut c_void, len: usize) {
    let pnt_ = pnt as *mut u8;
    let mut i: usize = 0;
    while i < len {
        core::ptr::write_volatile(pnt_.add(i), 0u8);
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_stackzero(_len: usize) {
    // HAVE_C_VARARRAYS / HAVE_ALLOCA not defined -> no-op
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_memcmp(b1_: *const c_void, b2_: *const c_void, len: usize) -> c_int {
    let b1 = b1_ as *const u8;
    let b2 = b2_ as *const u8;
    let mut d: u8 = 0;
    for i in 0..len {
        d |= core::ptr::read_volatile(b1.add(i)) ^ core::ptr::read_volatile(b2.add(i));
    }
    (1i32 & (((d as i32) - 1) >> 8)) - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_compare(b1_: *const u8, b2_: *const u8, len: usize) -> c_int {
    let b1 = b1_;
    let b2 = b2_;
    let mut gt: u8 = 0;
    let mut eq: u8 = 1;
    let mut i = len;
    while i != 0 {
        i -= 1;
        let x1 = core::ptr::read_volatile(b1.add(i)) as u16;
        let x2 = core::ptr::read_volatile(b2.add(i)) as u16;
        gt |= ((((x2 as u32).wrapping_sub(x1 as u32)) >> 8) as u8) & eq;
        eq &= (((((x2 ^ x1) as u32).wrapping_sub(1)) >> 8) as u8);
    }
    (gt as i32 + gt as i32 + eq as i32) - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int {
    let mut d: u8 = 0;
    for i in 0..nlen {
        d |= *n.add(i);
    }
    (1i32 & (((d as i32) - 1) >> 8)) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_increment(n: *mut u8, nlen: usize) {
    let mut c: u16 = 1;
    for i in 0..nlen {
        c += *n.add(i) as u16;
        *n.add(i) = c as u8;
        c >>= 8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_add(a: *mut u8, b: *const u8, len: usize) {
    let mut c: u16 = 0;
    for i in 0..len {
        c += (*a.add(i) as u16) + (*b.add(i) as u16);
        *a.add(i) = c as u8;
        c >>= 8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_sub(a: *mut u8, b: *const u8, len: usize) {
    let mut c: u16 = 0;
    for i in 0..len {
        c = (*a.add(i) as u16)
            .wrapping_sub(*b.add(i) as u16)
            .wrapping_sub(c);
        *a.add(i) = c as u8;
        c = (c >> 8) & 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_alloc_init() -> c_int {
    // HAVE_ALIGNED_MALLOC not defined
    randombytes_buf(
        core::ptr::addr_of_mut!(CANARY) as *mut c_void,
        CANARY_SIZE,
    );
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_mlock(_addr: *mut c_void, _len: usize) -> c_int {
    *libc::__errno_location() = libc::ENOSYS;
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_munlock(addr: *mut c_void, len: usize) -> c_int {
    sodium_memzero(addr, len);
    *libc::__errno_location() = libc::ENOSYS;
    -1
}

// No HAVE_ALIGNED_MALLOC: simple malloc/free.
unsafe fn _sodium_malloc(size: usize) -> *mut c_void {
    libc::malloc(if size > 0 { size } else { 1 })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_malloc(size: usize) -> *mut c_void {
    let ptr = _sodium_malloc(size);
    if ptr.is_null() {
        return core::ptr::null_mut();
    }
    libc::memset(ptr, GARBAGE_VALUE, size);
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_allocarray(count: usize, size: usize) -> *mut c_void {
    if count > 0 && size >= usize::MAX / count {
        *libc::__errno_location() = libc::ENOMEM;
        return core::ptr::null_mut();
    }
    sodium_malloc(count * size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_free(ptr: *mut c_void) {
    libc::free(ptr);
}

// No HAVE_PAGE_PROTECTION -> ENOSYS
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_mprotect_noaccess(_ptr: *mut c_void) -> c_int {
    *libc::__errno_location() = libc::ENOSYS;
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_mprotect_readonly(_ptr: *mut c_void) -> c_int {
    *libc::__errno_location() = libc::ENOSYS;
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_mprotect_readwrite(_ptr: *mut c_void) -> c_int {
    *libc::__errno_location() = libc::ENOSYS;
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_pad(
    padded_buflen_p: *mut usize,
    buf: *mut u8,
    unpadded_buflen: usize,
    blocksize: usize,
    max_buflen: usize,
) -> c_int {
    let mut xpadlen: usize;
    let xpadded_len: usize;
    let mut mask: u8;
    let mut barrier_mask: u8;

    if blocksize == 0 {
        return -1;
    }
    xpadlen = blocksize - 1;
    if (blocksize & (blocksize - 1)) == 0 {
        xpadlen -= unpadded_buflen & (blocksize - 1);
    } else {
        xpadlen -= unpadded_buflen % blocksize;
    }
    if usize::MAX - unpadded_buflen <= xpadlen {
        sodium_misuse();
    }
    xpadded_len = unpadded_buflen + xpadlen;
    if xpadded_len >= max_buflen {
        return -1;
    }
    let tail = buf.add(xpadded_len);
    if !padded_buflen_p.is_null() {
        *padded_buflen_p = xpadded_len + 1;
    }
    mask = 0;
    let shift = (core::mem::size_of::<usize>() - 1) * 8;
    for i in 0..blocksize {
        barrier_mask = (((i ^ xpadlen).wrapping_sub(1)) >> shift) as u8;
        let p = tail.offset(-(i as isize));
        *p = (*p & mask) | (0x80 & barrier_mask);
        mask |= barrier_mask;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_unpad(
    unpadded_buflen_p: *mut usize,
    buf: *const u8,
    padded_buflen: usize,
    blocksize: usize,
) -> c_int {
    let mut acc: u8 = 0;
    let mut valid: u8 = 0;
    let mut pad_len: usize = 0;

    if padded_buflen < blocksize || blocksize == 0 {
        return -1;
    }
    let tail = buf.add(padded_buflen - 1);

    for i in 0..blocksize {
        let c = *tail.offset(-(i as isize));
        let is_barrier: usize = ((((acc as usize).wrapping_sub(1))
            & (pad_len.wrapping_sub(1))
            & (((c ^ 0x80) as usize).wrapping_sub(1)))
            >> 8)
            & 1;
        acc |= c;
        pad_len |= i & (1usize.wrapping_add(!is_barrier));
        valid |= is_barrier as u8;
    }
    *unpadded_buflen_p = padded_buflen - 1 - pad_len;

    (valid.wrapping_sub(1)) as i8 as c_int
}
