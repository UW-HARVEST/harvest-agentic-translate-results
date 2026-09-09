//! Translation of `sodium/utils.c`
//!
//! The reference build defines no `HAVE_*` feature macros, therefore:
//!   * `HAVE_ALIGNED_MALLOC` is undefined  -> plain `malloc()`/`free()` are used
//!     and the guarded-allocation machinery is compiled out.
//!   * `HAVE_PAGE_PROTECTION` is undefined -> `_sodium_mprotect()` sets ENOSYS.
//!   * `HAVE_MLOCK`/`HAVE_MADVISE` are undefined -> mlock/munlock set ENOSYS.
//!   * `HAVE_WEAK_SYMBOLS` is undefined    -> the volatile-loop variants of
//!     memzero/memcmp/compare are used.
//!   * Neither `HAVE_C_VARARRAYS` nor `HAVE_ALLOCA` is defined, so
//!     `sodium_stackzero()` has an empty body.

use core::ffi::{c_int, c_void};
use core::ptr;

const CANARY_SIZE: usize = 16;
const GARBAGE_VALUE: u8 = 0xdb;

static mut CANARY: [u8; CANARY_SIZE] = [0; CANARY_SIZE];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_memzero(pnt: *mut c_void, len: usize) {
    let pnt_ = pnt as *mut u8;
    let mut i: usize = 0;

    while i < len {
        ptr::write_volatile(pnt_.add(i), 0u8);
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_stackzero(_len: usize) {
    /* no-op: neither HAVE_C_VARARRAYS nor HAVE_ALLOCA is defined */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_memcmp(b1_: *const c_void, b2_: *const c_void, len: usize) -> c_int {
    let b1 = b1_ as *const u8;
    let b2 = b2_ as *const u8;
    let mut d: u8 = 0;

    for i in 0..len {
        d |= ptr::read_volatile(b1.add(i)) ^ ptr::read_volatile(b2.add(i));
    }
    (1 & ((d as c_int).wrapping_sub(1) >> 8)) - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_compare(b1_: *const u8, b2_: *const u8, len: usize) -> c_int {
    let b1 = b1_;
    let b2 = b2_;
    let mut gt: u8 = 0;
    let mut eq: u8 = 1;

    let mut i = len;
    let mut x1v: u16;
    let mut x2v: u16;
    while i != 0 {
        i -= 1;
        x1v = ptr::read_volatile(b1.add(i)) as u16;
        x2v = ptr::read_volatile(b2.add(i)) as u16;
        gt |= (((x2v as u32).wrapping_sub(x1v as u32) >> 8) as u8) & eq;
        eq &= (((x2v ^ x1v) as u32).wrapping_sub(1) >> 8) as u8;
    }
    (gt as c_int) + (gt as c_int) + (eq as c_int) - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int {
    let mut d: u8 = 0;

    for i in 0..nlen {
        d |= *n.add(i);
    }
    1 & ((d as c_int).wrapping_sub(1) >> 8)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_increment(n: *mut u8, nlen: usize) {
    let mut c: usize = 1;

    for i in 0..nlen {
        c += *n.add(i) as usize;
        *n.add(i) = c as u8;
        c >>= 8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_add(a: *mut u8, b: *const u8, len: usize) {
    let mut c: usize = 0;

    for i in 0..len {
        c += (*a.add(i) as usize) + (*b.add(i) as usize);
        *a.add(i) = c as u8;
        c >>= 8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_sub(a: *mut u8, b: *const u8, len: usize) {
    // `uint_fast16_t` is 64-bit wide on LP64 targets; the C code relies on the
    // borrow propagating through `(c >> 8) & 1`.
    let mut c: u64 = 0;

    for i in 0..len {
        c = (*a.add(i) as u64)
            .wrapping_sub(*b.add(i) as u64)
            .wrapping_sub(c);
        *a.add(i) = c as u8;
        c = (c >> 8) & 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _sodium_alloc_init() -> c_int {
    unsafe {
        crate::randombytes::randombytes_buf(
            (&raw mut CANARY) as *mut c_void,
            CANARY_SIZE,
        );
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_mlock(_addr: *mut c_void, _len: usize) -> c_int {
    crate::set_errno(crate::ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_munlock(addr: *mut c_void, len: usize) -> c_int {
    sodium_memzero(addr, len);
    crate::set_errno(crate::ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_malloc(size: usize) -> *mut c_void {
    let ptr = _sodium_malloc(size);
    if ptr.is_null() {
        return ptr::null_mut();
    }
    crate::common::memset(ptr as *mut u8, GARBAGE_VALUE, size);

    ptr
}

unsafe fn _sodium_malloc(size: usize) -> *mut c_void {
    libc::malloc(if size > 0 { size } else { 1 })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_allocarray(count: usize, size: usize) -> *mut c_void {
    if count > 0 && size >= usize::MAX / count {
        crate::set_errno(crate::ENOMEM);
        return ptr::null_mut();
    }
    sodium_malloc(count * size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_free(ptr: *mut c_void) {
    libc::free(ptr);
}

/// `_sodium_mprotect()` without `HAVE_PAGE_PROTECTION`.
fn _sodium_mprotect() -> c_int {
    crate::set_errno(crate::ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_mprotect_noaccess(_ptr: *mut c_void) -> c_int {
    _sodium_mprotect()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_mprotect_readonly(_ptr: *mut c_void) -> c_int {
    _sodium_mprotect()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_mprotect_readwrite(_ptr: *mut c_void) -> c_int {
    _sodium_mprotect()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_pad(
    padded_buflen_p: *mut usize,
    buf: *mut u8,
    unpadded_buflen: usize,
    blocksize: usize,
    max_buflen: usize,
) -> c_int {
    let tail: *mut u8;
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
        crate::sodium_core::sodium_misuse();
    }
    xpadded_len = unpadded_buflen + xpadlen;
    if xpadded_len >= max_buflen {
        return -1;
    }
    tail = buf.add(xpadded_len);
    if !padded_buflen_p.is_null() {
        *padded_buflen_p = xpadded_len + 1;
    }
    mask = 0;
    for i in 0..blocksize {
        barrier_mask = ((i ^ xpadlen).wrapping_sub(1) >> ((core::mem::size_of::<usize>() - 1) * 8))
            as u8;
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
    let tail: *const u8;
    let mut acc: u8 = 0;
    let mut c: u8;
    let mut valid: u8 = 0;
    let mut pad_len: usize = 0;
    let mut is_barrier: usize;

    if padded_buflen < blocksize || blocksize == 0 {
        return -1;
    }
    tail = buf.add(padded_buflen - 1);

    for i in 0..blocksize {
        c = *tail.offset(-(i as isize));
        is_barrier = (((acc as usize).wrapping_sub(1)
            & pad_len.wrapping_sub(1)
            & ((c ^ 0x80) as usize).wrapping_sub(1))
            >> 8)
            & 1;
        acc |= c;
        pad_len |= i & (1usize.wrapping_add(!is_barrier));
        valid |= is_barrier as u8;
    }
    *unpadded_buflen_p = padded_buflen - 1 - pad_len;

    // C: `return (int) (valid - 1U);` -- `valid` is promoted to `unsigned int`,
    // so a zero `valid` yields 0xFFFFFFFF, i.e. -1.
    (valid as u32).wrapping_sub(1) as c_int
}
