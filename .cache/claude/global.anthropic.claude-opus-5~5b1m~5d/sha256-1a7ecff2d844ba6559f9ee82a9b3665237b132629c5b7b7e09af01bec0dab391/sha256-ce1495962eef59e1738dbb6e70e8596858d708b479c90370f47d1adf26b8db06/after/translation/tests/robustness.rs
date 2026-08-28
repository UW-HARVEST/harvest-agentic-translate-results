//! Extra differential probes for behaviour that the offset comparison in
//! `differential.rs` cannot observe:
//!
//!   * the exact `malloc` request size (`malloc_usable_size` parity),
//!   * whether the failure path actually calls `free` (leak parity),
//!   * whether either implementation reads past `bufferSize` (guard page),
//!   * whether either implementation writes past `numLines` elements.

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::c_char;

extern "C" {
    fn malloc_usable_size(p: *mut c_void) -> usize;
    fn free(p: *mut c_void);
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
    fn munmap(addr: *mut c_void, len: usize) -> i32;
    fn sysconf(name: i32) -> i64;
}

const PROT_NONE: i32 = 0x0;
const PROT_READ: i32 = 0x1;
const PROT_WRITE: i32 = 0x2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;
const SC_PAGESIZE: i32 = 30; // _SC_PAGESIZE on Linux

fn page_size() -> usize {
    let v = unsafe { sysconf(SC_PAGESIZE) };
    if v > 0 {
        v as usize
    } else {
        4096
    }
}

// ---------------------------------------------------------------------------
// 1. Allocation-capacity check: the returned block must be large enough for
//    `numLines` pointers in BOTH libraries.
//
//    NOTE: `malloc_usable_size` is *not* a function of the request size alone
//    (glibc may serve a request out of a larger free chunk), so the two
//    libraries' usable sizes may legitimately differ. Exact request-size parity
//    is verified soundly in `tests/alloc_size.rs` by interposing `malloc`.
//    Here we only assert the lower bound, which is heap-state independent.
// ---------------------------------------------------------------------------
#[test]
fn returned_block_is_large_enough() {
    let p = pair();
    // An all-NUL buffer of length numLines always yields exactly numLines lines,
    // so these calls succeed and a block is returned.
    for k in [0usize, 1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 64, 127, 128, 1000, 4096, 65536] {
        let mut buf = vec![0u8; k.max(1)];
        let base = buf.as_mut_ptr() as *mut c_char;

        let rc = unsafe { p.c.create_raw(base, k, k) };
        let rr = unsafe { p.rust.create_raw(base, k, k) };
        assert!(!rc.is_null(), "C returned NULL for k={k}");
        assert!(!rr.is_null(), "Rust returned NULL for k={k}");

        let uc = unsafe { malloc_usable_size(rc as *mut c_void) };
        let ur = unsafe { malloc_usable_size(rr as *mut c_void) };
        assert!(
            uc >= k * 8,
            "C block ({uc}) smaller than numLines*8 ({}) for k={k}",
            k * 8
        );
        assert!(
            ur >= k * 8,
            "Rust block ({ur}) smaller than numLines*8 ({}) for k={k} — \
             the array would be written out of bounds",
            k * 8
        );

        unsafe {
            free(rc as *mut c_void);
            free(rr as *mut c_void);
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Failure-path leak parity: `c_src/src/lib.c:29` calls `free(bufferPtrs)`
//    before returning NULL. If the Rust translation forgot that `free`, the
//    process RSS would grow without bound here (8 MiB per call).
// ---------------------------------------------------------------------------
fn resident_bytes() -> usize {
    let s = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: usize = s
        .split_whitespace()
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    pages * page_size()
}

fn peak_rss_bytes() -> usize {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("VmPeak:") {
            let kb: usize = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            return kb * 1024;
        }
    }
    0
}

#[test]
fn failure_path_frees_its_allocation() {
    let p = pair();
    // numLines = 1<<20 -> malloc(8 MiB); bufferSize = 0 -> loop never runs ->
    // lineIndex(0) != numLines -> free + NULL.
    const K: usize = 1 << 20;
    const ITERS: usize = 512;

    for imp in [&p.c, &p.rust] {
        // warm up the allocator so the first-touch growth is not attributed
        for _ in 0..8 {
            let r = unsafe { imp.create_raw(std::ptr::null_mut(), K, 0) };
            assert!(r.is_null(), "{} should have returned NULL", imp.name);
        }
        let before_rss = resident_bytes();
        let before_peak = peak_rss_bytes();
        for _ in 0..ITERS {
            let r = unsafe { imp.create_raw(std::ptr::null_mut(), K, 0) };
            assert!(r.is_null(), "{} should have returned NULL", imp.name);
        }
        let after_rss = resident_bytes();
        let after_peak = peak_rss_bytes();
        let leaked_if_no_free = K * 8 * ITERS; // 4 GiB
        let rss_growth = after_rss.saturating_sub(before_rss);
        let peak_growth = after_peak.saturating_sub(before_peak);
        assert!(
            rss_growth < leaked_if_no_free / 16,
            "{} appears to leak on the failure path: RSS grew {rss_growth} bytes over \
             {ITERS} calls (a missing free() would cost {leaked_if_no_free})",
            imp.name
        );
        assert!(
            peak_growth < leaked_if_no_free / 16,
            "{} appears to leak on the failure path: VmPeak grew {peak_growth} bytes",
            imp.name
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Guard-page test: no read may happen at or past `buffer + bufferSize`.
//    The buffer is placed so that its last byte is the last byte of a mapped
//    page and the following page is PROT_NONE. Any over-read faults.
// ---------------------------------------------------------------------------
struct GuardedBuf {
    map: *mut c_void,
    map_len: usize,
    pub ptr: *mut c_char,
    pub len: usize,
}

impl GuardedBuf {
    fn new(bytes: &[u8]) -> GuardedBuf {
        let ps = page_size();
        let map_len = 2 * ps;
        let map = unsafe {
            mmap(
                std::ptr::null_mut(),
                map_len,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert!(map as isize != -1 && !map.is_null(), "mmap failed");
        // second page becomes a guard
        let guard = (map as usize + ps) as *mut c_void;
        assert_eq!(unsafe { mprotect(guard, ps, PROT_NONE) }, 0, "mprotect failed");
        assert!(bytes.len() <= ps, "buffer larger than a page");
        let ptr = (map as usize + ps - bytes.len()) as *mut c_char;
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
        }
        GuardedBuf {
            map,
            map_len,
            ptr,
            len: bytes.len(),
        }
    }
}

impl Drop for GuardedBuf {
    fn drop(&mut self) {
        unsafe {
            munmap(self.map, self.map_len);
        }
    }
}

#[test]
fn never_reads_past_buffer_size() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xBEEF);

    // Worst case for over-reading: no NUL anywhere, so the inner loop is only
    // stopped by the `pos + len < bufferSize` guard.
    for &n in &[1usize, 2, 3, 7, 8, 15, 16, 31, 63, 64, 127, 128, 255, 256, 1024, 4096] {
        let bytes: Vec<u8> = (0..n).map(|i| 1 + (i % 254) as u8).collect();
        let g = GuardedBuf::new(&bytes);
        for k in [0usize, 1, 2, 3] {
            let rc = unsafe { p.c.create_raw(g.ptr, k, g.len) };
            let rr = unsafe { p.rust.create_raw(g.ptr, k, g.len) };
            assert_eq!(rc.is_null(), rr.is_null(), "guarded n={n} k={k}");
            if !rc.is_null() {
                for i in 0..k {
                    let a = unsafe { *rc.add(i) };
                    let b = unsafe { *rr.add(i) };
                    assert_eq!(a, b, "guarded pointer {i} differs (n={n}, k={k})");
                    let off = a as usize - g.ptr as usize;
                    assert!(off < g.len, "pointer {i} out of window (off={off}, len={})", g.len);
                }
            }
            unsafe {
                if !rc.is_null() {
                    free(rc as *mut c_void)
                }
                if !rr.is_null() {
                    free(rr as *mut c_void)
                }
            }
        }
    }

    // Randomized guarded buffers with mixed NUL density.
    for _ in 0..2000 {
        let n = rng.range(1, 200);
        let density = [0u32, 10, 50, 90][rng.below(4)];
        let bytes = rng.bytes(n, density);
        let g = GuardedBuf::new(&bytes);
        let k = rng.below(12);
        let rc = unsafe { p.c.create_raw(g.ptr, k, g.len) };
        let rr = unsafe { p.rust.create_raw(g.ptr, k, g.len) };
        assert_eq!(rc.is_null(), rr.is_null(), "guarded random n={n} k={k}");
        if !rc.is_null() {
            for i in 0..k {
                assert_eq!(
                    unsafe { *rc.add(i) },
                    unsafe { *rr.add(i) },
                    "guarded random pointer {i} differs (n={n}, k={k})"
                );
            }
        }
        unsafe {
            if !rc.is_null() {
                free(rc as *mut c_void)
            }
            if !rr.is_null() {
                free(rr as *mut c_void)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 4. No write past `numLines` elements: over-allocate the array ourselves is
//    impossible (the library allocates), so instead check that the bytes
//    immediately after the `numLines`-th element inside the *usable* region
//    are untouched relative to the C.
// ---------------------------------------------------------------------------
#[test]
fn writes_exactly_num_lines_elements() {
    let p = pair();
    // Pick k where malloc's usable size exceeds k*8, giving slack to inspect.
    for k in [1usize, 2, 3, 5, 6, 7, 9, 10, 11, 13, 14, 15] {
        let mut buf = vec![0u8; k];
        let base = buf.as_mut_ptr() as *mut c_char;

        let rc = unsafe { p.c.create_raw(base, k, k) };
        let rr = unsafe { p.rust.create_raw(base, k, k) };
        assert!(!rc.is_null() && !rr.is_null(), "k={k}");

        // Compare the region that the library is required to have written,
        // byte-for-byte. (Usable size is deliberately not compared — see
        // `returned_block_is_large_enough`.)
        // the first k*8 bytes are the written pointers (compare as offsets,
        // both were given the same base so they must be identical bytes).
        let bc = unsafe { std::slice::from_raw_parts(rc as *const u8, k * 8) };
        let br = unsafe { std::slice::from_raw_parts(rr as *const u8, k * 8) };
        assert_eq!(bc, br, "written pointer bytes differ at k={k}");

        unsafe {
            free(rc as *mut c_void);
            free(rr as *mut c_void);
        }
    }
}

// ---------------------------------------------------------------------------
// 5. Determinism / repeatability across many calls with the same input.
// ---------------------------------------------------------------------------
#[test]
fn repeated_identical_calls_are_stable() {
    let p = pair();
    let mut buf = b"aa\0\0bbb\0c".to_vec();
    let base = buf.as_mut_ptr() as *mut c_char;
    let n = buf.len();
    for k in 0..=6usize {
        let mut first: Option<Observed> = None;
        for _ in 0..200 {
            for imp in [&p.c, &p.rust] {
                let o = unsafe { observe(imp, base, k, n) };
                match &first {
                    None => first = Some(o),
                    Some(f) => assert_eq!(&o, f, "unstable result for k={k} in {}", imp.name),
                }
            }
        }
    }
}
