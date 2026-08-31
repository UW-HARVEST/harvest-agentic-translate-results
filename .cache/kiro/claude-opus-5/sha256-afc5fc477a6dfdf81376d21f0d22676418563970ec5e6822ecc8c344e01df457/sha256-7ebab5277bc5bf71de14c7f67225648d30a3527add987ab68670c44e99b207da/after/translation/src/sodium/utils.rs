//! Translation of `libsodium/sodium/utils.c`
//!
//! The reference build defines no `HAVE_*` macros. In particular
//! `HAVE_ALIGNED_MALLOC` / `HAVE_PAGE_PROTECTION` / `HAVE_MLOCK` /
//! `HAVE_MPROTECT` / `HAVE_C_VARARRAYS` / `HAVE_ALLOCA` /
//! `HAVE_WEAK_SYMBOLS` are all undefined, which selects the plain
//! `malloc`/`free` guarded-allocation fallbacks and the `ENOSYS` stubs.

use core::ffi::{c_int, c_void};
use core::ptr;

use crate::plat::{set_errno, ENOMEM, ENOSYS};

const CANARY_SIZE: usize = 16;
const GARBAGE_VALUE: u8 = 0xdb;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

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
    // Neither HAVE_C_VARARRAYS nor HAVE_ALLOCA is defined: the function body
    // is empty in the reference build.
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_memcmp(b1_: *const c_void, b2_: *const c_void, len: usize) -> c_int {
    let b1 = b1_ as *const u8;
    let b2 = b2_ as *const u8;
    let mut d: u8 = 0;
    let mut i: usize = 0;
    while i < len {
        d |= *b1.add(i) ^ *b2.add(i);
        i += 1;
    }
    (1 & ((d as c_int - 1) >> 8)) - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_compare(b1: *const u8, b2: *const u8, len: usize) -> c_int {
    let mut gt: u8 = 0;
    let mut eq: u8 = 1;
    let mut i = len;
    while i != 0 {
        i -= 1;
        let x1 = *b1.add(i) as u16;
        let x2 = *b2.add(i) as u16;
        gt |= ((((x2 as u32).wrapping_sub(x1 as u32)) >> 8) & eq as u32) as u8;
        eq &= ((((x2 ^ x1) as u32).wrapping_sub(1)) >> 8) as u8;
    }
    (gt as c_int + gt as c_int + eq as c_int) - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int {
    let mut d: u8 = 0;
    let mut i: usize = 0;
    while i < nlen {
        d |= *n.add(i);
        i += 1;
    }
    1 & ((d as c_int - 1) >> 8)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_increment(n: *mut u8, nlen: usize) {
    let mut c: u64 = 1;
    for i in 0..nlen {
        c += *n.add(i) as u64;
        *n.add(i) = c as u8;
        c >>= 8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_add(a: *mut u8, b: *const u8, len: usize) {
    let mut c: u64 = 0;
    for i in 0..len {
        c += *a.add(i) as u64 + *b.add(i) as u64;
        *a.add(i) = c as u8;
        c >>= 8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_sub(a: *mut u8, b: *const u8, len: usize) {
    let mut c: u64 = 0;
    for i in 0..len {
        c = (*a.add(i) as u64).wrapping_sub(*b.add(i) as u64).wrapping_sub(c);
        *a.add(i) = c as u8;
        c = (c >> 8) & 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _sodium_alloc_init() -> c_int {
    unsafe {
        randombytes_buf(
            ptr::addr_of_mut!(CANARY) as *mut c_void,
            CANARY_SIZE,
        );
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_mlock(_addr: *mut c_void, _len: usize) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_munlock(addr: *mut c_void, len: usize) -> c_int {
    sodium_memzero(addr, len);
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_malloc(size: usize) -> *mut c_void {
    let ptr = unsafe { malloc(if size > 0 { size } else { 1 }) };
    if ptr.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        ptr::write_bytes(ptr as *mut u8, GARBAGE_VALUE, size);
    }
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_allocarray(count: usize, size: usize) -> *mut c_void {
    if count > 0 && size >= usize::MAX / count {
        set_errno(ENOMEM);
        return ptr::null_mut();
    }
    sodium_malloc(count.wrapping_mul(size))
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_free(ptr: *mut c_void) {
    unsafe { free(ptr) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_mprotect_noaccess(_ptr: *mut c_void) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_mprotect_readonly(_ptr: *mut c_void) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_mprotect_readwrite(_ptr: *mut c_void) -> c_int {
    set_errno(ENOSYS);
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
    if blocksize == 0 {
        return -1;
    }
    let mut xpadlen: usize = blocksize - 1;
    if (blocksize & (blocksize - 1)) == 0 {
        xpadlen -= unpadded_buflen & (blocksize - 1);
    } else {
        xpadlen -= unpadded_buflen % blocksize;
    }
    if usize::MAX - unpadded_buflen <= xpadlen {
        crate::sodium::core::sodium_misuse();
    }
    let xpadded_len = unpadded_buflen + xpadlen;
    if xpadded_len >= max_buflen {
        return -1;
    }
    let tail = buf.add(xpadded_len);
    if !padded_buflen_p.is_null() {
        *padded_buflen_p = xpadded_len + 1;
    }
    let mut mask: u8 = 0;
    for i in 0..blocksize {
        let barrier_mask = (((i ^ xpadlen).wrapping_sub(1)) >> ((core::mem::size_of::<usize>() - 1) * 8)) as u8;
        let p = tail.sub(i);
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
        let c = *tail.sub(i);
        let is_barrier: usize = ((((acc as u32).wrapping_sub(1)) as usize
            & pad_len.wrapping_sub(1)
            & (((c as u32 ^ 0x80).wrapping_sub(1)) as usize))
            >> 8)
            & 1;
        acc |= c;
        pad_len |= i & (1usize.wrapping_add(!is_barrier));
        valid |= is_barrier as u8;
    }
    *unpadded_buflen_p = padded_buflen - 1 - pad_len;

    ((valid as u32).wrapping_sub(1)) as c_int
}
