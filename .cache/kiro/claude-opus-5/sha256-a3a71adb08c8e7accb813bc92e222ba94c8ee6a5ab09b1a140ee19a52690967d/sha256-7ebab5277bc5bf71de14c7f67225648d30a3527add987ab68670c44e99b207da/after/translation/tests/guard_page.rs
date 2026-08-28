//! Over-read detection: places the NUL terminator on the last byte of a mapped
//! page followed by an unmapped guard page, so any read past the terminator
//! faults instead of silently succeeding.
//!
//! Both libraries are exercised the same way; if the Rust translation copied
//! more than `strlen + 1` bytes it would segfault here while the C original
//! would not.

mod common;

use std::ffi::c_char;
use std::ffi::c_void;

use common::Both;
use common::free;
use common::snapshot;

const PROT_NONE: i32 = 0x0;
const PROT_READ: i32 = 0x1;
const PROT_WRITE: i32 = 0x2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;

unsafe extern "C" {
    fn mmap(
        addr: *mut c_void,
        length: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
    fn munmap(addr: *mut c_void, length: usize) -> i32;
}

/// A readable page immediately followed by an inaccessible guard page.
struct GuardedRegion {
    base: *mut u8,
    page: usize,
}

impl GuardedRegion {
    fn new() -> Self {
        let page = 4096usize;

        // Two pages: the first stays readable/writable, the second becomes a trap.
        // SAFETY: standard anonymous mapping request; result is checked below.
        let base = unsafe {
            mmap(
                std::ptr::null_mut(),
                2 * page,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert!(
            base as isize != -1 && !base.is_null(),
            "mmap of two pages failed"
        );

        // SAFETY: `base + page` is the second page of the mapping we just made.
        let rc = unsafe { mprotect(base.cast::<u8>().add(page).cast::<c_void>(), page, PROT_NONE) };
        assert_eq!(rc, 0, "mprotect of the guard page failed");

        Self {
            base: base.cast::<u8>(),
            page,
        }
    }

    /// Writes `payload` plus a NUL so the terminator is the final readable byte
    /// of the first page, and returns a pointer to the start of the payload.
    fn place_at_page_end(&self, payload: &[u8]) -> *const c_char {
        assert!(
            payload.len() + 1 <= self.page,
            "payload must fit in one page"
        );
        let offset = self.page - (payload.len() + 1);

        // SAFETY: `offset .. offset + payload.len() + 1` lies inside the first
        // (readable/writable) page of the mapping.
        unsafe {
            let start = self.base.add(offset);
            std::ptr::copy_nonoverlapping(payload.as_ptr(), start, payload.len());
            *start.add(payload.len()) = 0;
            start as *const c_char
        }
    }
}

impl Drop for GuardedRegion {
    fn drop(&mut self) {
        // SAFETY: unmaps exactly the region created in `new`.
        unsafe {
            munmap(self.base.cast::<c_void>(), 2 * self.page);
        }
    }
}

#[test]
fn no_read_past_terminator() {
    let both = Both::load();
    let region = GuardedRegion::new();

    for len in [0usize, 1, 2, 3, 7, 8, 15, 16, 17, 31, 32, 63, 64, 100, 1000, 4094, 4095] {
        let payload: Vec<u8> = (0..len).map(|i| ((i % 255) + 1) as u8).collect();
        let arg = region.place_at_page_end(&payload);

        // SAFETY: `arg` is NUL-terminated on the last readable byte of the page.
        let c_out = unsafe { both.c_strdup()(arg) };
        // SAFETY: as above; a read past the NUL would fault on the guard page.
        let rust_out = unsafe { both.rust_strdup()(arg) };

        assert!(!c_out.is_null(), "C returned NULL for len {len}");
        assert!(!rust_out.is_null(), "Rust returned NULL for len {len}");

        // SAFETY: both buffers hold `len + 1` bytes.
        let c_bytes = unsafe { snapshot(c_out, len) };
        // SAFETY: as above.
        let rust_bytes = unsafe { snapshot(rust_out, len) };

        assert_eq!(c_bytes, rust_bytes, "outputs differ at page-end len {len}");
        assert_eq!(&c_bytes[..len], &payload[..], "payload mismatch at len {len}");
        assert_eq!(c_bytes[len], 0, "missing terminator at len {len}");

        // SAFETY: both pointers came from `malloc`.
        unsafe {
            free(c_out as *mut c_void);
            free(rust_out as *mut c_void);
        }
    }
}
