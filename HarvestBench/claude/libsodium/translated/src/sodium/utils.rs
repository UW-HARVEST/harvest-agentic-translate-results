//! `sodium/utils.c`
//!
//! With no `HAVE_*` macros defined:
//!  * `HAVE_ALIGNED_MALLOC` is **not** defined (needs WINAPI_DESKTOP, or
//!    MAP_ANON+HAVE_MMAP, or HAVE_POSIX_MEMALIGN) so `sodium_malloc()` is the
//!    plain `malloc()` variant, `sodium_free()` is `free()`, and
//!    `sodium_mprotect_*()` all set `ENOSYS` and return -1.
//!  * `HAVE_WEAK_SYMBOLS` is not defined, so `sodium_memzero()` uses the
//!    volatile byte loop and the `_sodium_dummy_symbol_*` helpers are absent.
//!  * `HAVE_C_VARARRAYS`/`HAVE_ALLOCA` are not defined, so `sodium_stackzero()`
//!    has an empty body.
//!  * `HAVE_MLOCK` is not defined, so mlock/munlock set `ENOSYS`, return -1.

use core::ffi::{c_int, c_void};
use core::ptr;

use crate::common::{ENOMEM, ENOSYS, SIZE_MAX, free, malloc, set_errno};
use crate::sodium::core::sodium_misuse;

const CANARY_SIZE: usize = 16;
const GARBAGE_VALUE: u8 = 0xdb;

static mut CANARY: [u8; CANARY_SIZE] = [0; CANARY_SIZE];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_memzero(pnt: *mut c_void, len: usize) {
    let pnt_ = pnt as *mut u8;
    let mut i: usize = 0;
    while i < len {
        unsafe { ptr::write_volatile(pnt_.add(i), 0u8) };
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_stackzero(_len: usize) {
    // HAVE_C_VARARRAYS / HAVE_ALLOCA undefined -> empty body
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_memcmp(b1_: *const c_void, b2_: *const c_void, len: usize) -> c_int {
    let b1 = b1_ as *const u8;
    let b2 = b2_ as *const u8;
    let mut d: u8 = 0;
    for i in 0..len {
        d |= unsafe { ptr::read_volatile(b1.add(i)) ^ ptr::read_volatile(b2.add(i)) };
    }
    (1 & ((d as c_int).wrapping_sub(1) >> 8)) - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_compare(b1_: *const u8, b2_: *const u8, len: usize) -> c_int {
    let mut gt: u8 = 0;
    let mut eq: u8 = 1;
    let mut i = len;
    while i != 0 {
        i -= 1;
        let x1 = unsafe { ptr::read_volatile(b1_.add(i)) } as u16;
        let x2 = unsafe { ptr::read_volatile(b2_.add(i)) } as u16;
        gt |= (((x2 as u32).wrapping_sub(x1 as u32) >> 8) as u8) & eq;
        eq &= ((((x2 ^ x1) as u32).wrapping_sub(1)) >> 8) as u8;
    }
    ((gt as c_int) + (gt as c_int) + (eq as c_int)) - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int {
    let mut d: u8 = 0;
    for i in 0..nlen {
        d |= unsafe { *n.add(i) };
    }
    1 & ((d as c_int).wrapping_sub(1) >> 8)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_increment(n: *mut u8, nlen: usize) {
    let mut c: usize = 1;
    for i in 0..nlen {
        c += unsafe { *n.add(i) } as usize;
        unsafe { *n.add(i) = c as u8 };
        c >>= 8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_add(a: *mut u8, b: *const u8, len: usize) {
    let mut c: usize = 0;
    for i in 0..len {
        c += unsafe { *a.add(i) } as usize + unsafe { *b.add(i) } as usize;
        unsafe { *a.add(i) = c as u8 };
        c >>= 8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_sub(a: *mut u8, b: *const u8, len: usize) {
    // uint_fast16_t on x86-64 glibc is unsigned long (64-bit); the sequence
    // c = a[i] - b[i] - c; a[i] = (uchar) c; c = (c >> 8) & 1;
    // is width independent because only the low bit of (c >> 8) is kept.
    let mut c: u64 = 0;
    for i in 0..len {
        c = (unsafe { *a.add(i) } as u64)
            .wrapping_sub(unsafe { *b.add(i) } as u64)
            .wrapping_sub(c);
        unsafe { *a.add(i) = c as u8 };
        c = (c >> 8) & 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _sodium_alloc_init() -> c_int {
    // HAVE_ALIGNED_MALLOC undefined: skip the page-size probing entirely.
    crate::randombytes::randombytes_buf(
        (&raw mut CANARY) as *mut c_void,
        CANARY_SIZE,
    );
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_mlock(_addr: *mut c_void, _len: usize) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_munlock(addr: *mut c_void, len: usize) -> c_int {
    unsafe { sodium_memzero(addr, len) };
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_malloc(size: usize) -> *mut c_void {
    let ptr = unsafe { malloc(if size > 0 { size } else { 1 }) };
    if ptr.is_null() {
        return ptr::null_mut();
    }
    if size != 0 {
        unsafe { ptr::write_bytes(ptr as *mut u8, GARBAGE_VALUE, size) };
    }
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_allocarray(count: usize, size: usize) -> *mut c_void {
    if count > 0 && size >= SIZE_MAX / count {
        set_errno(ENOMEM);
        return ptr::null_mut();
    }
    sodium_malloc(count.wrapping_mul(size))
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_free(ptr: *mut c_void) {
    unsafe { free(ptr) };
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
    if SIZE_MAX - unpadded_buflen <= xpadlen {
        sodium_misuse();
    }
    let xpadded_len = unpadded_buflen + xpadlen;
    if xpadded_len >= max_buflen {
        return -1;
    }
    let tail = unsafe { buf.add(xpadded_len) };
    if !padded_buflen_p.is_null() {
        unsafe { *padded_buflen_p = xpadded_len + 1 };
    }
    let mut mask: u8 = 0;
    for i in 0..blocksize {
        let barrier_mask =
            ((i ^ xpadlen).wrapping_sub(1) >> ((core::mem::size_of::<usize>() - 1) * 8)) as u8;
        let p = unsafe { tail.sub(i) };
        unsafe { *p = (*p & mask) | (0x80 & barrier_mask) };
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
    let tail = unsafe { buf.add(padded_buflen - 1) };

    for i in 0..blocksize {
        let c = unsafe { *tail.sub(i) };
        // (( (acc - 1U) & (pad_len - 1U) & ((c ^ 0x80) - 1U) ) >> 8) & 1U
        // The C operands promote to size_t because of pad_len.
        let is_barrier: usize = (((acc as usize).wrapping_sub(1)
            & pad_len.wrapping_sub(1)
            & ((c ^ 0x80) as usize).wrapping_sub(1))
            >> 8)
            & 1;
        acc |= c;
        pad_len |= i & (1usize.wrapping_add(!is_barrier));
        valid |= is_barrier as u8;
    }
    unsafe { *unpadded_buflen_p = padded_buflen - 1 - pad_len };

    valid.wrapping_sub(1) as i8 as c_int
}
