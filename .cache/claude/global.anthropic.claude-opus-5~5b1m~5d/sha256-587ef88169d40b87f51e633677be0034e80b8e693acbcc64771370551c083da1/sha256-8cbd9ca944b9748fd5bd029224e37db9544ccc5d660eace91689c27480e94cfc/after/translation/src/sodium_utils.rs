//! Translation of `c_src/libsodium/sodium/utils.c`.
//!
//! The reference build defines none of the `HAVE_*` feature macros this file
//! checks (`HAVE_MEMSET_S`, `HAVE_EXPLICIT_BZERO`, `HAVE_MEMSET_EXPLICIT`,
//! `HAVE_EXPLICIT_MEMSET`, `HAVE_WEAK_SYMBOLS`, `HAVE_C_VARARRAYS`,
//! `HAVE_ALLOCA`, `HAVE_AMD64_ASM`, `HAVE_MPROTECT`, `HAVE_MLOCK`,
//! `HAVE_MADVISE`, `HAVE_ALIGNED_MALLOC` (and therefore not
//! `HAVE_PAGE_PROTECTION` either), `_WIN32`/`WINAPI_DESKTOP`), so every
//! function below is the portable fallback that survives preprocessing
//! (confirmed with `tools/cpp.sh`):
//!  * `sodium_memzero`   -> plain volatile-style byte loop.
//!  * `sodium_stackzero` -> empty body (neither vararrays nor alloca).
//!  * `sodium_memcmp` / `sodium_compare` -> the non-weak-symbols branch.
//!  * `sodium_increment` / `sodium_add` / `sodium_sub` -> no inline asm,
//!    plain byte-at-a-time loops.
//!  * `_sodium_alloc_init` -> `HAVE_ALIGNED_MALLOC` not defined, so only the
//!    `randombytes_buf(canary, CANARY_SIZE)` call survives.
//!  * `sodium_mlock` / `sodium_munlock` -> no madvise/mlock: `errno = ENOSYS`.
//!  * `_mprotect_noaccess/readonly/readwrite` -> no `HAVE_MPROTECT`:
//!    `errno = ENOSYS`.
//!  * `_sodium_malloc` / `sodium_free` -> the plain (non-guarded-heap)
//!    `malloc`/`free` path.
//!  * `_sodium_mprotect` -> `HAVE_PAGE_PROTECTION` not defined: `errno = ENOSYS`.
//!
//! `sodium_bin2hex`/`sodium_hex2bin`/`sodium_bin2base64`/`sodium_base642bin`/
//! `sodium_base64_encoded_len`/`sodium_ip2bin`/`sodium_bin2ip` are declared in
//! `utils.h` but defined in `sodium/codecs.c`, not here (see
//! `_cbuild/persym.txt`), so they are not reproduced in this module.

use core::ffi::{c_int, c_void};

use crate::csys;

extern "C" {
    fn sodium_misuse() -> !;
    fn randombytes_buf(buf: *mut u8, size: usize);
}

const CANARY_SIZE: usize = 16;
const GARBAGE_VALUE: c_int = 0xdb;
const DEFAULT_PAGE_SIZE: usize = 0x10000;

// Kept for fidelity with the C source; not read anywhere in this build
// configuration since every `page_size`-consuming branch lives inside
// `#ifdef HAVE_ALIGNED_MALLOC`, which is not defined here.
static mut page_size: usize = DEFAULT_PAGE_SIZE;

static mut canary: [u8; CANARY_SIZE] = [0u8; CANARY_SIZE];

#[no_mangle]
pub unsafe extern "C" fn sodium_memzero(pnt: *mut c_void, len: usize) {
    let pnt_ = pnt as *mut u8;
    let mut i: usize = 0;

    while i < len {
        *pnt_.add(i) = 0u8;
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn sodium_stackzero(_len: usize) {
    // Neither HAVE_C_VARARRAYS nor HAVE_ALLOCA is defined, so the C body is
    // entirely empty.
}

#[no_mangle]
pub unsafe extern "C" fn sodium_memcmp(b1_: *const c_void, b2_: *const c_void, len: usize) -> c_int {
    let b1 = b1_ as *const u8;
    let b2 = b2_ as *const u8;
    let mut d: u8 = 0;
    let mut i: usize = 0;

    while i < len {
        d |= *b1.add(i) ^ *b2.add(i);
        i += 1;
    }
    (1i32 & (((d as i32) - 1) >> 8)) - 1
}

#[no_mangle]
pub unsafe extern "C" fn sodium_compare(b1_: *const u8, b2_: *const u8, len: usize) -> c_int {
    let b1 = b1_;
    let b2 = b2_;
    let mut gt: u8 = 0;
    let mut eq: u8 = 1;
    let mut i: usize = len;

    while i != 0 {
        i -= 1;
        let x1: u16 = *b1.add(i) as u16;
        let x2: u16 = *b2.add(i) as u16;
        gt |= ((((x2 as u32).wrapping_sub(x1 as u32)) >> 8) & (eq as u32)) as u8;
        eq &= ((((x2 as u32) ^ (x1 as u32)).wrapping_sub(1)) >> 8) as u8;
    }
    (gt as i32 + gt as i32 + eq as i32) - 1
}

#[no_mangle]
pub unsafe extern "C" fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int {
    let mut d: u8 = 0;
    let mut i: usize = 0;

    while i < nlen {
        d |= *n.add(i);
        i += 1;
    }
    1i32 & (((d as i32) - 1) >> 8)
}

#[no_mangle]
pub unsafe extern "C" fn sodium_increment(n: *mut u8, nlen: usize) {
    let mut i: usize = 0;
    let mut c: u32 = 1;

    while i < nlen {
        c = c.wrapping_add(*n.add(i) as u32);
        *n.add(i) = c as u8;
        c >>= 8;
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn sodium_add(a: *mut u8, b: *const u8, len: usize) {
    let mut c: u32 = 0;
    let mut i: usize = 0;

    while i < len {
        c = c
            .wrapping_add(*a.add(i) as u32)
            .wrapping_add(*b.add(i) as u32);
        *a.add(i) = c as u8;
        c >>= 8;
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn sodium_sub(a: *mut u8, b: *const u8, len: usize) {
    let mut c: u32 = 0;
    let mut i: usize = 0;

    while i < len {
        c = (*a.add(i) as u32).wrapping_sub(*b.add(i) as u32).wrapping_sub(c);
        *a.add(i) = c as u8;
        c = (c >> 8) & 1u32;
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_alloc_init() -> c_int {
    randombytes_buf((&raw mut canary) as *mut u8, CANARY_SIZE);

    0
}

#[no_mangle]
pub unsafe extern "C" fn sodium_mlock(_addr: *mut c_void, _len: usize) -> c_int {
    csys::set_errno(csys::ENOSYS);
    -1
}

#[no_mangle]
pub unsafe extern "C" fn sodium_munlock(addr: *mut c_void, len: usize) -> c_int {
    sodium_memzero(addr, len);
    csys::set_errno(csys::ENOSYS);
    -1
}

type MprotectCb = unsafe extern "C" fn(*mut c_void, usize) -> c_int;

unsafe extern "C" fn mprotect_noaccess(ptr: *mut c_void, size: usize) -> c_int {
    let _ = (ptr, size);
    csys::set_errno(csys::ENOSYS);
    -1
}

unsafe extern "C" fn mprotect_readonly(ptr: *mut c_void, size: usize) -> c_int {
    let _ = (ptr, size);
    csys::set_errno(csys::ENOSYS);
    -1
}

unsafe extern "C" fn mprotect_readwrite(ptr: *mut c_void, size: usize) -> c_int {
    let _ = (ptr, size);
    csys::set_errno(csys::ENOSYS);
    -1
}

unsafe fn sodium_malloc_raw(size: usize) -> *mut c_void {
    csys::malloc(if size > 0 { size } else { 1 })
}

#[no_mangle]
pub unsafe extern "C" fn sodium_malloc(size: usize) -> *mut c_void {
    let ptr = sodium_malloc_raw(size);
    if ptr.is_null() {
        return core::ptr::null_mut();
    }
    csys::memset(ptr, GARBAGE_VALUE, size);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn sodium_allocarray(count: usize, size: usize) -> *mut c_void {
    if count > 0 && size >= usize::MAX / count {
        csys::set_errno(csys::ENOMEM);
        return core::ptr::null_mut();
    }
    sodium_malloc(count.wrapping_mul(size))
}

#[no_mangle]
pub unsafe extern "C" fn sodium_free(ptr: *mut c_void) {
    csys::free(ptr);
}

unsafe fn sodium_mprotect_impl(ptr: *mut c_void, cb: MprotectCb) -> c_int {
    let _ = (ptr, cb);
    csys::set_errno(csys::ENOSYS);
    -1
}

#[no_mangle]
pub unsafe extern "C" fn sodium_mprotect_noaccess(ptr: *mut c_void) -> c_int {
    sodium_mprotect_impl(ptr, mprotect_noaccess)
}

#[no_mangle]
pub unsafe extern "C" fn sodium_mprotect_readonly(ptr: *mut c_void) -> c_int {
    sodium_mprotect_impl(ptr, mprotect_readonly)
}

#[no_mangle]
pub unsafe extern "C" fn sodium_mprotect_readwrite(ptr: *mut c_void) -> c_int {
    sodium_mprotect_impl(ptr, mprotect_readwrite)
}

#[no_mangle]
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
    let mut xpadlen = blocksize - 1;
    if (blocksize & (blocksize - 1)) == 0 {
        xpadlen = xpadlen.wrapping_sub(unpadded_buflen & (blocksize - 1));
    } else {
        xpadlen = xpadlen.wrapping_sub(unpadded_buflen % blocksize);
    }
    if usize::MAX.wrapping_sub(unpadded_buflen) <= xpadlen {
        sodium_misuse();
    }
    let xpadded_len = unpadded_buflen.wrapping_add(xpadlen);
    if xpadded_len >= max_buflen {
        return -1;
    }
    let tail = buf.wrapping_add(xpadded_len);
    if !padded_buflen_p.is_null() {
        *padded_buflen_p = xpadded_len.wrapping_add(1);
    }
    let mut mask: u8 = 0;
    let mut i: usize = 0;
    while i < blocksize {
        let barrier_mask: u8 = ((i ^ xpadlen).wrapping_sub(1)
            >> ((core::mem::size_of::<usize>() - 1) * 8)) as u8;
        let p = tail.wrapping_sub(i);
        *p = (*p & mask) | (0x80 & barrier_mask);
        mask |= barrier_mask;
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn sodium_unpad(
    unpadded_buflen_p: *mut usize,
    buf: *const u8,
    padded_buflen: usize,
    blocksize: usize,
) -> c_int {
    if padded_buflen < blocksize || blocksize == 0 {
        return -1;
    }
    let tail = buf.wrapping_add(padded_buflen.wrapping_sub(1));
    let mut acc: u8 = 0;
    let mut valid: u8 = 0;
    let mut pad_len: usize = 0;
    let mut i: usize = 0;

    while i < blocksize {
        let c = *tail.wrapping_sub(i);
        let is_barrier: usize = (((acc as usize).wrapping_sub(1))
            & (pad_len.wrapping_sub(1))
            & (((c as usize) ^ 0x80).wrapping_sub(1)))
            >> 8
            & 1;
        acc |= c;
        pad_len |= i & (1usize.wrapping_add(!is_barrier));
        valid |= is_barrier as u8;
        i += 1;
    }
    *unpadded_buflen_p = padded_buflen.wrapping_sub(1).wrapping_sub(pad_len);

    (valid as i32) - 1
}
