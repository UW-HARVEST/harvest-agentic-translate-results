//! Translation of `c_src/libsodium/sodium/utils.c`.
//!
//! The reference build has no `config.h`, so none of `HAVE_MMAP`,
//! `HAVE_MPROTECT`, `HAVE_MLOCK`, `HAVE_MADVISE`, `HAVE_POSIX_MEMALIGN`,
//! `HAVE_ALIGNED_MALLOC`, `HAVE_WEAK_SYMBOLS`, `HAVE_INLINE_ASM`,
//! `HAVE_C_VARARRAYS`, `HAVE_ALLOCA`, `HAVE_MEMSET_S`, `HAVE_EXPLICIT_BZERO`,
//! `HAVE_MEMSET_EXPLICIT`, `HAVE_EXPLICIT_MEMSET`, `HAVE_SYSCONF` are defined,
//! and `_WIN32`/`_MSC_VER` are undefined.  Consequently:
//!
//!   * `sodium_memzero()` uses the `volatile unsigned char *volatile` loop.
//!   * `sodium_stackzero()` has an empty body.
//!   * `sodium_memcmp()` / `sodium_compare()` use the `volatile` operands.
//!   * `sodium_mlock()` / `sodium_munlock()` set `errno = ENOSYS` and return -1.
//!   * `HAVE_ALIGNED_MALLOC` is *not* defined, so `_sodium_malloc()` is the
//!     plain `malloc()` wrapper, `sodium_free()` is plain `free()`, and
//!     `_sodium_mprotect()` sets `errno = ENOSYS` and returns -1.
//!   * `PAGE_SIZE` is not visible, so `DEFAULT_PAGE_SIZE` is `0x10000`.
//!
//! (Confirmed by running `gcc -E` over the file with the reference include
//! paths.)

use crate::common::*;
use core::ffi::{c_int, c_uint, c_void};

/* -------------------------------------------------------------------------
 * #defines
 * ------------------------------------------------------------------------- */

const CANARY_SIZE: usize = 16;
const GARBAGE_VALUE: c_int = 0xdb;

/* `#ifndef DEFAULT_PAGE_SIZE / #ifdef PAGE_SIZE ... #else 0x10000` — PAGE_SIZE
 * is not exposed by <limits.h>/<sys/param.h> in the reference build. */
const DEFAULT_PAGE_SIZE: usize = 0x10000;

/* <errno.h> values on Linux/glibc. */
const ENOSYS: c_int = 38;
const ENOMEM: c_int = 12;

/* <stdint.h> */
const SIZE_MAX: usize = usize::MAX;

/* CHAR_BIT */
const CHAR_BIT: usize = 8;

/* -------------------------------------------------------------------------
 * Externals (libc + other translation units)
 * ------------------------------------------------------------------------- */

extern "C" {
    fn __errno_location() -> *mut c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);

    /* randombytes/randombytes.c */
    fn randombytes_buf(buf: *mut c_void, size: usize);

    /* sodium/core.c — declared `__attribute__((noreturn))` */
    fn sodium_misuse() -> !;
}

#[inline(always)]
unsafe fn set_errno(e: c_int) {
    *__errno_location() = e;
}

/* -------------------------------------------------------------------------
 * File-scope statics
 * ------------------------------------------------------------------------- */

static mut page_size: usize = DEFAULT_PAGE_SIZE;
static mut canary: [u8; CANARY_SIZE] = [0u8; CANARY_SIZE];

/* -------------------------------------------------------------------------
 * sodium_memzero / sodium_stackzero
 * ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_memzero(pnt: *mut c_void, len: usize) {
    let pnt_: *mut u8 = pnt as *mut u8;
    let mut i: usize = 0;

    while i < len {
        core::ptr::write_volatile(pnt_.add(i), 0u8);
        i += 1;
    }
    /* Belt and braces: keep the stores from being reordered/elided. */
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_stackzero(len: usize) {
    /* Neither HAVE_C_VARARRAYS nor HAVE_ALLOCA is defined: empty body. */
    let _ = len;
}

/* -------------------------------------------------------------------------
 * Constant-time comparisons
 * ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_memcmp(
    b1_: *const c_void,
    b2_: *const c_void,
    len: usize,
) -> c_int {
    let b1: *const u8 = b1_ as *const u8;
    let b2: *const u8 = b2_ as *const u8;
    let mut i: usize;
    let mut d: u8 = 0;
    let d_p = core::ptr::addr_of_mut!(d);

    i = 0;
    while i < len {
        let x1 = core::ptr::read_volatile(b1.add(i));
        let x2 = core::ptr::read_volatile(b2.add(i));
        let acc = core::ptr::read_volatile(d_p);
        core::ptr::write_volatile(d_p, acc | (x1 ^ x2));
        i += 1;
    }
    let d = core::ptr::read_volatile(d_p);

    (1 & ((d as c_int - 1) >> 8)) - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_compare(
    b1_: *const u8,
    b2_: *const u8,
    len: usize,
) -> c_int {
    let b1: *const u8 = b1_;
    let b2: *const u8 = b2_;
    let mut i: usize;
    let mut gt: u8 = 0;
    let mut eq: u8 = 1;
    let gt_p = core::ptr::addr_of_mut!(gt);
    let eq_p = core::ptr::addr_of_mut!(eq);

    i = len;
    while i != 0 {
        i -= 1;
        let x1: u16 = core::ptr::read_volatile(b1.add(i)) as u16;
        let x2: u16 = core::ptr::read_volatile(b2.add(i)) as u16;

        /* gt |= (((unsigned int) x2 - (unsigned int) x1) >> 8) & eq; */
        let cur_gt = core::ptr::read_volatile(gt_p);
        let cur_eq = core::ptr::read_volatile(eq_p);
        let t: c_uint =
            (((x2 as c_uint).wrapping_sub(x1 as c_uint)) >> 8) & (cur_eq as c_uint);
        core::ptr::write_volatile(gt_p, ((cur_gt as c_uint) | t) as u8);

        /* eq &= (((unsigned int) (x2 ^ x1)) - 1) >> 8; */
        let cur_eq = core::ptr::read_volatile(eq_p);
        let t: c_uint = (((x2 ^ x1) as c_uint).wrapping_sub(1)) >> 8;
        core::ptr::write_volatile(eq_p, ((cur_eq as c_uint) & t) as u8);
    }
    let gt = core::ptr::read_volatile(gt_p);
    let eq = core::ptr::read_volatile(eq_p);

    (gt as c_int + gt as c_int + eq as c_int) - 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int {
    let mut i: usize;
    let mut d: u8 = 0;
    let d_p = core::ptr::addr_of_mut!(d);

    i = 0;
    while i < nlen {
        let acc = core::ptr::read_volatile(d_p);
        core::ptr::write_volatile(d_p, acc | *n.add(i));
        i += 1;
    }
    let d = core::ptr::read_volatile(d_p);

    1 & ((d as c_int - 1) >> 8)
}

/* -------------------------------------------------------------------------
 * Little-endian arithmetic on byte strings
 * ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_increment(n: *mut u8, nlen: usize) {
    let mut i: usize = 0;
    /* uint_fast16_t is `unsigned long` on LP64 glibc. */
    let mut c: u64 = 1;

    while i < nlen {
        c = c.wrapping_add(*n.add(i) as u64);
        *n.add(i) = c as u8;
        c >>= 8;
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_add(a: *mut u8, b: *const u8, len: usize) {
    let mut i: usize;
    let mut c: u64 = 0;

    i = 0;
    while i < len {
        c = c.wrapping_add((*a.add(i) as u64).wrapping_add(*b.add(i) as u64));
        *a.add(i) = c as u8;
        c >>= 8;
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_sub(a: *mut u8, b: *const u8, len: usize) {
    let mut c: u64 = 0;
    let mut i: usize;

    i = 0;
    while i < len {
        c = (*a.add(i) as u64)
            .wrapping_sub(*b.add(i) as u64)
            .wrapping_sub(c);
        *a.add(i) = c as u8;
        c = (c >> 8) & 1;
        i += 1;
    }
}

/* -------------------------------------------------------------------------
 * Allocator support
 * ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_alloc_init() -> c_int {
    /* HAVE_ALIGNED_MALLOC undefined: the page-size probe is compiled out. */
    randombytes_buf(core::ptr::addr_of_mut!(canary) as *mut c_void, CANARY_SIZE);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_mlock(addr: *mut c_void, len: usize) -> c_int {
    let _ = (addr, len);
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_munlock(addr: *mut c_void, len: usize) -> c_int {
    sodium_memzero(addr, len);
    set_errno(ENOSYS);
    -1
}

/* `static int _mprotect_*(void *ptr, size_t size)` — their addresses are taken
 * and handed to `_sodium_mprotect()`, hence the `extern "C"` fn type. */
unsafe extern "C" fn _mprotect_noaccess(ptr: *mut c_void, size: usize) -> c_int {
    let _ = (ptr, size);
    set_errno(ENOSYS);
    -1
}

unsafe extern "C" fn _mprotect_readonly(ptr: *mut c_void, size: usize) -> c_int {
    let _ = (ptr, size);
    set_errno(ENOSYS);
    -1
}

unsafe extern "C" fn _mprotect_readwrite(ptr: *mut c_void, size: usize) -> c_int {
    let _ = (ptr, size);
    set_errno(ENOSYS);
    -1
}

/* !HAVE_ALIGNED_MALLOC variant. */
unsafe fn _sodium_malloc(size: usize) -> *mut c_void {
    malloc(if size > 0 { size } else { 1 })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_malloc(size: usize) -> *mut c_void {
    let ptr: *mut c_void = _sodium_malloc(size);

    if ptr.is_null() {
        return core::ptr::null_mut();
    }
    memset(ptr as *mut u8, GARBAGE_VALUE as u8, size);

    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_allocarray(count: usize, size: usize) -> *mut c_void {
    if count > 0 && size >= SIZE_MAX / count {
        set_errno(ENOMEM);
        return core::ptr::null_mut();
    }
    sodium_malloc(count.wrapping_mul(size))
}

/* !HAVE_ALIGNED_MALLOC variant. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_free(ptr: *mut c_void) {
    free(ptr);
}

type MprotectCb = unsafe extern "C" fn(*mut c_void, usize) -> c_int;

/* !HAVE_PAGE_PROTECTION variant. */
unsafe fn _sodium_mprotect(ptr: *mut c_void, cb: MprotectCb) -> c_int {
    let _ = ptr;
    let _ = cb;
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_mprotect_noaccess(ptr: *mut c_void) -> c_int {
    _sodium_mprotect(ptr, _mprotect_noaccess)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_mprotect_readonly(ptr: *mut c_void) -> c_int {
    _sodium_mprotect(ptr, _mprotect_readonly)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_mprotect_readwrite(ptr: *mut c_void) -> c_int {
    _sodium_mprotect(ptr, _mprotect_readwrite)
}

/* -------------------------------------------------------------------------
 * ISO/IEC 7816-4 padding
 * ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_pad(
    padded_buflen_p: *mut usize,
    buf: *mut u8,
    unpadded_buflen: usize,
    blocksize: usize,
    max_buflen: usize,
) -> c_int {
    let tail: *mut u8;
    let mut i: usize;
    let mut xpadlen: usize;
    let xpadded_len: usize;
    let mut mask: u8;
    let mut barrier_mask: u8;

    if blocksize == 0 {
        return -1;
    }
    xpadlen = blocksize.wrapping_sub(1);
    if (blocksize & blocksize.wrapping_sub(1)) == 0 {
        xpadlen = xpadlen.wrapping_sub(unpadded_buflen & blocksize.wrapping_sub(1));
    } else {
        xpadlen = xpadlen.wrapping_sub(unpadded_buflen % blocksize);
    }
    if SIZE_MAX - unpadded_buflen <= xpadlen {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    xpadded_len = unpadded_buflen.wrapping_add(xpadlen);
    if xpadded_len >= max_buflen {
        return -1;
    }
    tail = buf.add(xpadded_len);
    if !padded_buflen_p.is_null() {
        *padded_buflen_p = xpadded_len.wrapping_add(1);
    }
    mask = 0;
    let mask_p = core::ptr::addr_of_mut!(mask);
    i = 0;
    while i < blocksize {
        barrier_mask = (((i ^ xpadlen).wrapping_sub(1))
            >> ((core::mem::size_of::<usize>() - 1) * CHAR_BIT)) as u8;
        let p = tail.offset(-(i as isize));
        let cur_mask = core::ptr::read_volatile(mask_p);
        *p = (*p & cur_mask) | (0x80 & barrier_mask);
        let cur_mask = core::ptr::read_volatile(mask_p);
        core::ptr::write_volatile(mask_p, cur_mask | barrier_mask);
        i += 1;
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
    let pad_len_p = core::ptr::addr_of_mut!(pad_len);
    let mut i: usize;
    let mut is_barrier: usize;

    if padded_buflen < blocksize || blocksize == 0 {
        return -1;
    }
    tail = buf.add(padded_buflen.wrapping_sub(1));

    i = 0;
    while i < blocksize {
        c = *tail.offset(-(i as isize));
        /* is_barrier =
         *   (((acc - 1U) & (pad_len - 1U) & ((c ^ 0x80) - 1U)) >> 8) & 1U;
         * `acc - 1U` and `(c ^ 0x80) - 1U` are `unsigned int` expressions that
         * are then zero-extended to `size_t` by the usual conversions. */
        let acc_m1: usize = (acc as c_uint).wrapping_sub(1) as usize;
        let pad_len_m1: usize = core::ptr::read_volatile(pad_len_p).wrapping_sub(1);
        let c_m1: usize = (((c as c_uint) ^ 0x80).wrapping_sub(1)) as usize;
        is_barrier = ((acc_m1 & pad_len_m1 & c_m1) >> 8) & 1;

        acc |= c;
        let cur_pad_len = core::ptr::read_volatile(pad_len_p);
        core::ptr::write_volatile(
            pad_len_p,
            cur_pad_len | (i & (!is_barrier).wrapping_add(1)),
        );
        valid |= is_barrier as u8;
        i += 1;
    }
    *unpadded_buflen_p = padded_buflen
        .wrapping_sub(1)
        .wrapping_sub(core::ptr::read_volatile(pad_len_p));

    (valid as c_uint).wrapping_sub(1) as c_int
}
