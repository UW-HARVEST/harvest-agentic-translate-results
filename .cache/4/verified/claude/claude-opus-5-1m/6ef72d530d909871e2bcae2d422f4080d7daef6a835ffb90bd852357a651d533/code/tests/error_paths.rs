//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1, E2) plus the generic FFI-boundary rows
//! (G1..G7). Both implementations are always invoked through their `.so`.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// E1 / G1 — `str == NULL` must return NULL from both
// ---------------------------------------------------------------------------
#[test]
fn err_e1_null_pointer() {
    let cf = c_strdup();
    let rf = rust_strdup();
    unsafe {
        let c = cf(std::ptr::null());
        let r = rf(std::ptr::null());
        assert!(c.is_null(), "E1: C returned {c:p} for NULL input");
        assert!(r.is_null(), "E1: Rust returned {r:p} for NULL input");
        assert_eq!(c, r, "E1: sentinel mismatch");
    }
}

#[test]
fn err_e1_null_repeated() {
    // The NULL path must be stateless: interleave with successful calls.
    let cf = c_strdup();
    let rf = rust_strdup();
    let mut rng = Rng::new(SEED ^ 0xE1);
    for i in 0..200 {
        unsafe {
            assert!(cf(std::ptr::null()).is_null(), "E1 iter {i}: C");
            assert!(rf(std::ptr::null()).is_null(), "E1 iter {i}: Rust");
        }
        let len = rng.below(40) as usize;
        let s = rng.nonzero_bytes(len);
        assert_same_dup(&s, &format!("E1 iter {i} interleaved ok call"));
    }
}

#[test]
fn err_e1_null_errno_untouched() {
    // The C code returns before touching libc, so errno must be preserved.
    let cf = c_strdup();
    let rf = rust_strdup();
    const SENTINEL: c_int = 0x4321;
    unsafe {
        *libc::__errno_location() = SENTINEL;
        let c = cf(std::ptr::null());
        let c_errno = *libc::__errno_location();

        *libc::__errno_location() = SENTINEL;
        let r = rf(std::ptr::null());
        let r_errno = *libc::__errno_location();

        assert!(c.is_null() && r.is_null());
        assert_eq!(c_errno, SENTINEL, "E1: C clobbered errno");
        assert_eq!(
            r_errno, c_errno,
            "E1: Rust errno {r_errno} != C errno {c_errno}"
        );
    }
}

// ---------------------------------------------------------------------------
// E2 — `malloc` failure must return NULL from both
//
// Reproduced in a forked child whose RLIMIT_AS is clamped below the current
// address-space size, so any fresh mmap/brk (and therefore the 64 MiB
// allocation the duplication needs) fails. Exit status encodes which
// implementation returned NULL: bit0 = C, bit1 = Rust.
// ---------------------------------------------------------------------------
#[test]
fn err_e2_malloc_failure_returns_null() {
    const BIG: usize = 64 << 20; // 64 MiB -> always served by mmap
    let cf = c_strdup();
    let rf = rust_strdup();

    // Allocate & fill the source in the parent so the child needs no allocation
    // before the calls under test.
    let mut src: Vec<u8> = vec![b'z'; BIG + 1];
    src[BIG] = 0;
    let src_ptr = src.as_ptr() as *const c_char;

    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "E2: fork failed");
        if pid == 0 {
            // ---- child: no allocation, no stdio, no atexit ----
            let lim = libc::rlimit {
                rlim_cur: 1 << 20, // 1 MiB, below current VmSize
                rlim_max: 1 << 20,
            };
            if libc::setrlimit(libc::RLIMIT_AS, &lim) != 0 {
                libc::_exit(64);
            }
            let c = cf(src_ptr);
            let r = rf(src_ptr);
            let mut code = 0;
            if c.is_null() {
                code |= 1;
            }
            if r.is_null() {
                code |= 2;
            }
            libc::_exit(code);
        }

        let mut status: c_int = 0;
        assert_eq!(libc::waitpid(pid, &mut status, 0), pid, "E2: waitpid failed");
        assert!(
            libc::WIFEXITED(status),
            "E2: child died abnormally (signal {})",
            libc::WTERMSIG(status)
        );
        let code = libc::WEXITSTATUS(status);
        assert_ne!(code, 64, "E2: setrlimit failed in child");
        assert_ne!(
            code, 0,
            "E2: neither implementation hit the malloc-failure path (test setup)"
        );
        assert_eq!(
            code, 3,
            "E2 DIVERGENCE: C returned NULL = {}, Rust returned NULL = {}",
            code & 1 != 0,
            code & 2 != 0
        );
    }
    // Keep the source alive until after the child ran.
    assert_eq!(src[BIG], 0);
}

// ---------------------------------------------------------------------------
// G2 — zero length ("" ) is *not* an error
// ---------------------------------------------------------------------------
#[test]
fn err_g2_empty_string_is_not_an_error() {
    let cf = c_strdup();
    let rf = rust_strdup();
    let src = b"\0";
    unsafe {
        let c = cf(src.as_ptr() as *const c_char);
        let r = rf(src.as_ptr() as *const c_char);
        assert!(!c.is_null(), "G2: C returned NULL for empty string");
        assert!(!r.is_null(), "G2: Rust returned NULL for empty string");
        assert_ne!(c as *const u8, src.as_ptr(), "G2: C aliased source");
        assert_ne!(r as *const u8, src.as_ptr(), "G2: Rust aliased source");
        assert_eq!(*c, 0);
        assert_eq!(*r, 0);
        libc::free(c as *mut libc::c_void);
        libc::free(r as *mut libc::c_void);
    }
}

// ---------------------------------------------------------------------------
// G3 — "oversized" inputs succeed identically (no truncation, no error)
// ---------------------------------------------------------------------------
#[test]
fn err_g3_oversized_input_succeeds() {
    let mut rng = Rng::new(SEED ^ 0xA3);
    for len in [(1usize << 20) - 1, 1 << 20, (1 << 20) + 1, (1 << 22) + 3] {
        let s = rng.nonzero_bytes(len);
        assert_same_dup(&s, &format!("G3 len {len}"));
    }
}

// ---------------------------------------------------------------------------
// G4 — never reads past the terminator (page-guard check)
// ---------------------------------------------------------------------------
#[test]
fn err_g4_no_read_past_terminator() {
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    unsafe {
        let total = page * 2;
        let base = libc::mmap(
            std::ptr::null_mut(),
            total,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        assert_ne!(base, libc::MAP_FAILED, "G4: mmap failed");
        let base = base as *mut u8;
        assert_eq!(
            libc::mprotect(base.add(page) as *mut libc::c_void, page, libc::PROT_NONE),
            0
        );
        for i in 0..page {
            *base.add(i) = b'A';
        }
        *base.add(page - 1) = 0; // NUL is the very last readable byte
        for len in [0usize, 1, 2, 3, 4, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128] {
            let src = base.add(page - 1 - len) as *const c_char;
            assert_same_dup_raw(src, len, &format!("G4 len {len}"));
        }
        libc::munmap(base as *mut libc::c_void, total);
    }
}

// ---------------------------------------------------------------------------
// G5 — no enum/flag parameters exist; probe surplus scalar arguments instead
// ---------------------------------------------------------------------------
#[test]
fn err_g5_no_enum_parameters() {
    let cfe = c_strdup_extra();
    let rfe = rust_strdup_extra();
    let src = b"enum-probe\0";
    // Values that would be "no valid variant" for any C enum parameter.
    for (a, b) in [
        (-1i32, i32::MIN),
        (i32::MAX, 0x7FFF_FFFE),
        (12345, -99999),
        (0, 0),
    ] {
        unsafe {
            let c = cfe(src.as_ptr() as *const c_char, a as c_int, b as c_int);
            let r = rfe(src.as_ptr() as *const c_char, a as c_int, b as c_int);
            assert!(!c.is_null() && !r.is_null(), "G5: NULL for ({a},{b})");
            let cgot = std::slice::from_raw_parts(c as *const u8, src.len());
            let rgot = std::slice::from_raw_parts(r as *const u8, src.len());
            assert_eq!(cgot, &src[..], "G5: C content for ({a},{b})");
            assert_eq!(rgot, cgot, "G5: Rust content for ({a},{b})");
            assert_eq!(libc::strlen(c), src.len() - 1, "G5: C strlen ({a},{b})");
            assert_eq!(libc::strlen(r), libc::strlen(c), "G5: Rust strlen ({a},{b})");
            libc::free(c as *mut libc::c_void);
            libc::free(r as *mut libc::c_void);
        }
        // Same probe on the NULL/error path.
        unsafe {
            let c = cfe(std::ptr::null(), a as c_int, b as c_int);
            let r = rfe(std::ptr::null(), a as c_int, b as c_int);
            assert!(c.is_null(), "G5: C non-NULL on NULL path ({a},{b})");
            assert!(r.is_null(), "G5: Rust non-NULL on NULL path ({a},{b})");
        }
    }
}

// ---------------------------------------------------------------------------
// G6 — misaligned / interior source pointer
// ---------------------------------------------------------------------------
#[test]
fn err_g6_unaligned_source() {
    let mut rng = Rng::new(SEED ^ 0xA6);
    for off in 1..=8usize {
        for len in [0usize, 1, 9, 17, 63, 64, 65, 511] {
            let mut buf = rng.nonzero_bytes(off);
            buf.extend_from_slice(&rng.nonzero_bytes(len));
            buf.push(0);
            unsafe {
                assert_same_dup_raw(
                    buf.as_ptr().add(off) as *const c_char,
                    len,
                    &format!("G6 off {off} len {len}"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// G7 — the result is an independent block on the C heap
// ---------------------------------------------------------------------------
#[test]
fn err_g7_result_is_independent_c_heap_block() {
    let mut rng = Rng::new(SEED ^ 0xA7);
    let cf = c_strdup();
    let rf = rust_strdup();
    for i in 0..100 {
        let len = 1 + rng.below(300) as usize;
        let mut s = rng.nonzero_bytes(len);
        s.push(0);
        let snapshot = s.clone();
        unsafe {
            let src = s.as_ptr() as *const c_char;
            let c = cf(src);
            let r = rf(src);
            assert!(!c.is_null() && !r.is_null(), "G7 iter {i}: NULL");
            // Mutate both copies; the source must be untouched.
            // (last byte first, so that for len == 1 the NUL write wins)
            *(c.add(len - 1) as *mut u8) = 0xFF;
            *c = 0;
            *(r.add(len - 1) as *mut u8) = 0xFF;
            *r = 0;
            assert_eq!(s, snapshot, "G7 iter {i}: source modified");
            assert_eq!(libc::strlen(c), 0, "G7: C block not writable as expected");
            assert_eq!(libc::strlen(r), 0, "G7: Rust block not writable as expected");
            // Must be releasable by the caller's free(): glibc aborts otherwise.
            libc::free(c as *mut libc::c_void);
            libc::free(r as *mut libc::c_void);
        }
    }
}
