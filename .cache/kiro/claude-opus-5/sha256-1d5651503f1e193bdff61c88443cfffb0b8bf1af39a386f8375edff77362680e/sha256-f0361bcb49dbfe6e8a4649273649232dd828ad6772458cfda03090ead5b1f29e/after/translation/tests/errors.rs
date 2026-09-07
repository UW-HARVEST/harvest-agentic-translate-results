//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact rejection
//! condition, calls BOTH the C `.so` and the Rust `.so` through `libloading`,
//! and asserts they return the SAME sentinel (this API's only error signal is a
//! `NULL` return — there is no error enum or numeric status code).
//!
//! Row 5 of `ERRORS.md` (out-of-range enum value across the FFI boundary) has no
//! executable test because the API has no enum/int/flag parameter at all:
//! `char *custom_strdup(const char *str)` takes a single pointer. The defined
//! domain of that pointer is `{NULL}` ∪ `{valid NUL-terminated buffers}`; both
//! halves are covered here and in `configs.rs`. Passing a non-NULL pointer that
//! is not a valid C string is undefined behaviour in the C (`strlen` over-reads),
//! so it is not a defined input and is deliberately not exercised.
//!
//! Row 7 (`strlen(str) + 1` overflowing `size_t`) requires a 2^64−1-byte object
//! and is unreachable on any real host; the Rust matches C's unsigned wraparound
//! by construction via `wrapping_add(1)`.

mod common;

use common::{Rng, SEED, assert_same, assert_same_bytes, c_impl, rust_impl};
use std::ffi::c_char;

/// Row 1 — `str == NULL` (`if(!str)` at `lib.c:11`): both must return NULL.
#[test]
fn err_row1_null_input_returns_null() {
    let c = c_impl();
    let r = rust_impl();
    for i in 0..1000 {
        let pc = unsafe { (c.custom_strdup)(std::ptr::null()) };
        let pr = unsafe { (r.custom_strdup)(std::ptr::null()) };
        assert!(pc.is_null(), "C returned {pc:p} for NULL input (iteration {i})");
        assert!(pr.is_null(), "Rust returned {pr:p} for NULL input (iteration {i})");
    }
    // Also via the shared differential helper.
    assert_same(std::ptr::null(), "err row1");
}

/// Row 2 — `malloc` failure (`if(!newstr)` at `lib.c:18`): both must return NULL.
///
/// Provoked in a forked child: allocate a 64 MiB string outside the heap, then
/// lower `RLIMIT_AS` to just above current usage so that the library's
/// `malloc(64 MiB + 1)` cannot be served. The child exits with a bitmask —
/// bit 0 set if C returned non-NULL, bit 1 set if Rust returned non-NULL — so
/// exit code 0 means both correctly returned the NULL sentinel.
#[test]
fn err_row2_malloc_failure_returns_null() {
    const STR_LEN: usize = 64 << 20; // 64 MiB of content
    const SLACK: usize = 4 << 20; //  4 MiB of headroom below the AS limit

    let c = c_impl();
    let r = rust_impl();

    // Warm up both implementations in the parent so every lazy PLT entry
    // (strlen/malloc/memcpy in each .so) is resolved before we fork; the child
    // must not need the dynamic loader.
    let warm = [b'x', 0u8];
    for imp in [c, r] {
        let p = unsafe { (imp.custom_strdup)(warm.as_ptr() as *const c_char) };
        assert!(!p.is_null());
        unsafe { libc::free(p as *mut libc::c_void) };
    }

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");

    if pid == 0 {
        // ---- child: only async-signal-safe-ish work, then _exit ----
        let map = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                STR_LEN + 1,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if map == libc::MAP_FAILED {
            unsafe { libc::_exit(64) };
        }
        let s = map as *mut u8;
        unsafe { libc::memset(map, b'A' as libc::c_int, STR_LEN) };
        unsafe { *s.add(STR_LEN) = 0 };

        // Current virtual size, straight from /proc, without allocating.
        let mut fd_buf = [0u8; 128];
        let path = b"/proc/self/statm\0";
        let fd = unsafe { libc::open(path.as_ptr() as *const c_char, libc::O_RDONLY) };
        if fd < 0 {
            unsafe { libc::_exit(65) };
        }
        let n = unsafe { libc::read(fd, fd_buf.as_mut_ptr() as *mut libc::c_void, fd_buf.len()) };
        unsafe { libc::close(fd) };
        if n <= 0 {
            unsafe { libc::_exit(66) };
        }
        let mut vm_pages: usize = 0;
        for &b in &fd_buf[..n as usize] {
            if b.is_ascii_digit() {
                vm_pages = vm_pages * 10 + (b - b'0') as usize;
            } else {
                break;
            }
        }
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
        let limit = vm_pages * page + SLACK;

        let rl = libc::rlimit { rlim_cur: limit as libc::rlim_t, rlim_max: limit as libc::rlim_t };
        if unsafe { libc::setrlimit(libc::RLIMIT_AS, &rl) } != 0 {
            unsafe { libc::_exit(67) };
        }

        // Now the library's malloc(64 MiB + 1) must fail in both .so's.
        let pc = unsafe { (c.custom_strdup)(s as *const c_char) };
        let pr = unsafe { (r.custom_strdup)(s as *const c_char) };
        let mut code = 0i32;
        if !pc.is_null() {
            code |= 1;
        }
        if !pr.is_null() {
            code |= 2;
        }
        unsafe { libc::_exit(code) };
    }

    // ---- parent ----
    let mut status: libc::c_int = 0;
    let w = unsafe { libc::waitpid(pid, &mut status, 0) };
    assert_eq!(w, pid, "waitpid failed");
    assert!(libc::WIFEXITED(status), "child did not exit normally (status {status:#x})");
    let code = libc::WEXITSTATUS(status);
    match code {
        0 => {} // both returned NULL — C and Rust agree on the sentinel
        1 => panic!("divergence: C returned non-NULL on malloc failure, Rust returned NULL"),
        2 => panic!("divergence: Rust returned non-NULL on malloc failure, C returned NULL"),
        3 => panic!(
            "test setup ineffective: neither implementation hit malloc failure \
             (RLIMIT_AS was not tight enough)"
        ),
        64..=67 => panic!("test setup failed in child (code {code})"),
        other => panic!("unexpected child exit code {other}"),
    }
}

/// Row 3 — the empty string is NOT an error: both must return a non-NULL 1-byte
/// buffer holding a single NUL.
#[test]
fn err_row3_empty_string_is_not_an_error() {
    let buf = [0u8];
    for imp in [c_impl(), rust_impl()] {
        for i in 0..256 {
            let p = unsafe { (imp.custom_strdup)(buf.as_ptr() as *const c_char) };
            assert!(!p.is_null(), "{} rejected the empty string (iteration {i})", imp.name);
            assert_eq!(unsafe { *p }, 0, "{} did not NUL-terminate the empty copy", imp.name);
            unsafe { libc::free(p as *mut libc::c_void) };
        }
    }
    let got = assert_same_bytes(&buf, "err row3").unwrap();
    assert_eq!(got, vec![0u8]);
}

/// Row 4 — oversized-but-allocatable length: both must succeed identically.
/// (The unservable end of this axis is `err_row2_malloc_failure_returns_null`.)
#[test]
fn err_row4_oversized_length() {
    let mut rng = Rng::new(SEED ^ 0xE4);
    for len in [16usize << 20, (16 << 20) + 1] {
        let buf = rng.cstring(len);
        let got = assert_same_bytes(&buf, &format!("err row4 len={len}")).unwrap();
        assert_eq!(got.len(), len + 1);
    }
}

/// Row 6 — byte-domain boundaries: min content byte `1`, max content byte `255`,
/// and a `0` byte truncating the copy exactly where `strlen` stops.
#[test]
fn err_row6_byte_domain_boundaries() {
    // Minimum and maximum content byte, at several lengths.
    for &b in &[1u8, 255u8] {
        for len in [1usize, 2, 8, 17, 64, 1000] {
            let mut v = vec![b; len];
            v.push(0);
            let got = assert_same_bytes(&v, &format!("err row6 byte={b} len={len}")).unwrap();
            assert_eq!(got.len(), len + 1);
            assert!(got[..len].iter().all(|x| *x == b));
        }
    }

    // A `0` byte is not content: it truncates. Terminator at every position of a
    // 64-byte buffer whose remaining bytes are 0xFF garbage.
    for nul_at in 0..64usize {
        let mut v = vec![0xFFu8; 64];
        v[nul_at] = 0;
        let got = assert_same_bytes(&v, &format!("err row6 truncate at {nul_at}")).unwrap();
        assert_eq!(got.len(), nul_at + 1, "copy must stop at the first 0 byte");
        assert_eq!(got[nul_at], 0);
    }

    // One step past the top of the byte domain is not representable in `u8`,
    // so the boundary is exhaustively covered by 1..=255 in configs row 9.
}
