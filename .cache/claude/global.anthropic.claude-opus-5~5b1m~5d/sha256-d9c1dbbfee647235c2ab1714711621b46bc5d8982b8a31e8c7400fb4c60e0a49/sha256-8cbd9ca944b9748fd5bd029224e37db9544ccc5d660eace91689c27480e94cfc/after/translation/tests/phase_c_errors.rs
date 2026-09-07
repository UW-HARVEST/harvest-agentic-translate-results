//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Each test constructs the exact rejection condition, calls BOTH `.so`s
//! through `libloading`, and asserts they return the SAME sentinel — namely a
//! pointer that compares exactly equal to `(char *)NULL`, not merely
//! "both failed somehow".

mod common;

use common::{CStrBuf, Rng, assert_both_null, assert_same, libs};
use std::ffi::{c_char, c_int, c_void};

const SEED: u64 = 0x5EED_1234_ABCD_0002;

// ---------------------------------------------------------------------------
// ERRORS.md row 1 / G1 — str == NULL  (lib.c:11-12)
// ---------------------------------------------------------------------------

#[test]
fn err01_null_pointer_returns_null() {
    assert_both_null(std::ptr::null(), "err01 NULL argument");
}

/// Row 1 again, hammered: `errno` must not be disturbed and the result must be
/// NULL every single time (no state, no one-shot behaviour).
#[test]
fn err01b_null_pointer_repeated_and_errno_untouched() {
    let l = libs();
    for i in 0..1000 {
        set_errno(0x4242);
        let cp = unsafe { (l.c)(std::ptr::null()) };
        let c_errno = errno();
        set_errno(0x4242);
        let rp = unsafe { (l.rs)(std::ptr::null()) };
        let r_errno = errno();

        assert_eq!(cp as usize, 0, "err01b i={i}: C returned non-NULL");
        assert_eq!(rp as usize, 0, "err01b i={i}: Rust returned non-NULL");
        assert_eq!(
            c_errno, r_errno,
            "err01b i={i}: errno differs after NULL rejection (C={c_errno}, Rust={r_errno})"
        );
        assert_eq!(c_errno, 0x4242, "err01b i={i}: C clobbered errno on the NULL path");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 — malloc() failure  (lib.c:17-18)
//
// Forced in a forked child: both libraries are dlopen'd in the parent BEFORE
// the fork (so no allocation is needed after the limit is applied), the child
// clamps RLIMIT_AS to just above its current address-space usage, then calls
// custom_strdup on a multi-megabyte string. malloc cannot satisfy that, so both
// implementations must take the `if(!newstr) return NULL;` branch.
//
// The child's exit code encodes which implementations returned NULL:
//   0 = both NULL (expected)   1 = C non-NULL   2 = Rust non-NULL
//   3 = both non-NULL (limit did not bite -> test is inconclusive, reported)
// ---------------------------------------------------------------------------

const RLIMIT_AS: c_int = 9; // Linux/x86-64

#[repr(C)]
struct RLimit {
    rlim_cur: u64,
    rlim_max: u64,
}

unsafe extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
    fn getrlimit(resource: c_int, rlim: *mut RLimit) -> c_int;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn open(path: *const c_char, flags: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn __errno_location() -> *mut c_int;
    fn malloc(n: usize) -> *mut c_void;
}

/// `malloc` reached through an opaque function pointer.
///
/// Calling the `malloc` symbol directly is NOT usable for OOM testing: LLVM
/// knows `malloc`'s semantics, so it deletes allocations whose result is not
/// observably used and then folds the `is_null()` check to `false`. Routing the
/// call through `black_box` hides the identity of the callee, forcing a real
/// call and a real null check.
fn opaque_malloc(n: usize) -> *mut c_void {
    let f: unsafe extern "C" fn(usize) -> *mut c_void = std::hint::black_box(malloc);
    std::hint::black_box(unsafe { f(std::hint::black_box(n)) })
}

fn errno() -> c_int {
    unsafe { *__errno_location() }
}
fn set_errno(v: c_int) {
    unsafe { *__errno_location() = v };
}

/// Allocation-free diagnostic writer usable after the RLIMIT clamp is applied.
fn child_report(label: &[u8], value: u64) {
    let mut buf = [0u8; 128];
    let mut i = 0usize;
    for &b in label {
        buf[i] = b;
        i += 1;
    }
    let mut digits = [0u8; 20];
    let mut d = 0usize;
    let mut v = value;
    if v == 0 {
        digits[0] = b'0';
        d = 1;
    }
    while v > 0 {
        digits[d] = b'0' + (v % 10) as u8;
        v /= 10;
        d += 1;
    }
    while d > 0 {
        d -= 1;
        buf[i] = digits[d];
        i += 1;
    }
    buf[i] = b'\n';
    i += 1;
    unsafe { write(2, buf.as_ptr() as *const c_void, i) };
}

/// Allocation-free read of /proc/self/statm field 0 (pages) -> bytes.
fn raw_vm_size_bytes() -> u64 {
    let path = b"/proc/self/statm\0";
    let fd = unsafe { open(path.as_ptr() as *const c_char, 0 /* O_RDONLY */) };
    if fd < 0 {
        return 0;
    }
    let mut buf = [0u8; 256];
    let n = unsafe { read(fd, buf.as_mut_ptr() as *mut c_void, buf.len()) };
    unsafe { close(fd) };
    if n <= 0 {
        return 0;
    }
    let mut pages: u64 = 0;
    for &b in &buf[..n as usize] {
        if b.is_ascii_digit() {
            pages = pages * 10 + (b - b'0') as u64;
        } else {
            break;
        }
    }
    pages * 4096
}

/// Current address-space size in bytes, from /proc/self/statm field 0 (pages).
fn vm_size_bytes() -> u64 {
    let s = std::fs::read_to_string("/proc/self/statm").expect("read /proc/self/statm");
    let pages: u64 = s.split_whitespace().next().unwrap().parse().unwrap();
    pages * 4096
}

#[test]
fn err02_malloc_failure_returns_null() {
    let l = libs(); // dlopen BOTH libraries before forking
    let c_fn = l.c;
    let rs_fn = l.rs;

    // A 32 MiB source string, allocated (and touched) before the fork.
    // Built with EXACT capacity so there is no realloc and therefore no large
    // free chunk left in the malloc arena that could satisfy the child's
    // request without asking the kernel for new address space.
    let len = 32 * 1024 * 1024usize;
    let mut body: Vec<u8> = Vec::with_capacity(len + 1);
    body.resize(len, b'A');
    body.push(0);
    assert_eq!(body.capacity(), len + 1, "source buffer must not have been reallocated");
    let p = body.as_ptr() as *const c_char;

    // Warm up libc paths (locale, malloc arenas, lazy PLT for strlen/memcpy)
    // so the child does not need fresh address space for anything but the copy.
    {
        let warm = CStrBuf::new(b"warmup");
        let a = unsafe { c_fn(warm.ptr()) };
        let b = unsafe { rs_fn(warm.ptr()) };
        unsafe {
            common::free(a as *mut _);
            common::free(b as *mut _);
        }
    }
    // Clamp the child's address space to barely above what it already uses, so
    // a fresh 32 MiB mapping cannot possibly be granted. Computed BEFORE the
    // limit is applied (reading /proc needs allocations of its own).
    let headroom = vm_size_bytes() + 1024 * 1024;

    // Scratch array for the child's arena-draining step, reserved in the PARENT
    // so it is already part of the child's address space (the child must not
    // need any new mapping once the limit is in place).
    const SLOTS: usize = 1 << 16;
    let mut blocks: Vec<*mut c_void> = vec![std::ptr::null_mut(); SLOTS];
    let blocks_ptr = blocks.as_mut_ptr();

    let _ = std::io::Write::flush(&mut std::io::stdout());

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed: errno={}", errno());

    if pid == 0 {
        // ---- child ----
        let rl = RLimit { rlim_cur: headroom, rlim_max: headroom };
        if unsafe { setrlimit(RLIMIT_AS, &rl) } != 0 {
            unsafe { _exit(4) };
        }
        // Drain whatever slack already sits free inside the malloc arena, from
        // coarse to fine, keeping every block. Afterwards the largest satisfiable
        // request is < 4 KiB, so the 32 MiB copy cannot possibly be served from
        // existing memory and must ask the kernel -- which RLIMIT_AS now refuses.
        let mut n = 0usize;
        for &step in &[1usize << 20, 64 * 1024, 4096] {
            loop {
                if n >= SLOTS {
                    break;
                }
                let q = opaque_malloc(step);
                if q.is_null() {
                    break;
                }
                unsafe { *blocks_ptr.add(n) = q };
                n += 1;
            }
        }
        // Sanity: a plain malloc of this size must now fail.
        let probe = opaque_malloc(len + 1);
        if !probe.is_null() {
            let mut chk = RLimit { rlim_cur: 0, rlim_max: 0 };
            unsafe { getrlimit(RLIMIT_AS, &mut chk) };
            // Diagnostics only reachable when the clamp failed to bite.
            child_report(b"err02: rlimit cur=", chk.rlim_cur);
            child_report(b"err02: drained  n=", n as u64);
            child_report(b"err02: headroom =", headroom);
            child_report(b"err02: child vm =", raw_vm_size_bytes());
            unsafe { _exit(5) };
        }
        let cp = unsafe { c_fn(p) };
        let rp = unsafe { rs_fn(p) };
        let code = match (cp.is_null(), rp.is_null()) {
            (true, true) => 0,
            (false, true) => 1,
            (true, false) => 2,
            (false, false) => 3,
        };
        unsafe { _exit(code) };
    }

    // ---- parent ----
    let mut status: c_int = 0;
    let w = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(w, pid, "waitpid failed");
    assert!(status & 0x7f == 0, "child died on a signal: status={status:#x}");
    let code = (status >> 8) & 0xff;

    match code {
        0 => { /* both returned NULL — row 2 verified */ }
        1 => panic!("err02: C returned non-NULL but Rust returned NULL under OOM"),
        2 => panic!("err02: Rust returned non-NULL but C returned NULL under OOM (DIVERGENCE)"),
        3 => panic!(
            "err02: BOTH returned non-NULL under the RLIMIT_AS clamp \
             (headroom={headroom} bytes, request={} bytes)",
            len + 1
        ),
        4 => panic!("err02: setrlimit failed in the child"),
        5 => panic!(
            "err02: RLIMIT_AS did not bite - probe malloc({}) succeeded with headroom={headroom}",
            len + 1
        ),
        other => panic!("err02: unexpected child exit code {other}"),
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 3 / G2 — empty string, the zero-length boundary (lib.c:14)
// ---------------------------------------------------------------------------

#[test]
fn err03_empty_string_zero_length_boundary() {
    let l = libs();
    let nul: [u8; 1] = [0];
    let p = nul.as_ptr() as *const c_char;
    let cp = unsafe { (l.c)(p) };
    let rp = unsafe { (l.rs)(p) };
    assert!(!cp.is_null(), "err03: C returned NULL for \"\"");
    assert!(!rp.is_null(), "err03: Rust returned NULL for \"\" but C did not");
    let cb = unsafe { *(cp as *const u8) };
    let rb = unsafe { *(rp as *const u8) };
    assert_eq!(cb, 0, "err03: C result is not \"\"");
    assert_eq!(rb, cb, "err03: Rust result byte differs from C");
    unsafe {
        common::free(cp as *mut _);
        common::free(rp as *mut _);
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 4 — the NUL is the LAST readable byte of the mapping.
//
// The body is placed at the very end of an mmap'd page followed by an
// unmapped guard page, so any read past the terminator would SIGSEGV. If either
// implementation over-reads by even one byte, this test crashes rather than
// silently passing.
// ---------------------------------------------------------------------------

const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const PROT_NONE: c_int = 0;
const MAP_PRIVATE: c_int = 2;
const MAP_ANONYMOUS: c_int = 0x20;

unsafe extern "C" {
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        off: i64,
    ) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
    fn munmap(addr: *mut c_void, len: usize) -> c_int;
}

#[test]
fn err04_terminator_at_last_readable_byte() {
    let l = libs();
    let page = 4096usize;
    // Two pages: [readable page][guard page with PROT_NONE].
    let base = unsafe {
        mmap(std::ptr::null_mut(), 2 * page, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0)
    };
    assert!(base as isize != -1, "mmap failed: errno={}", errno());
    let guard = unsafe { (base as *mut u8).add(page) } as *mut c_void;
    assert_eq!(unsafe { mprotect(guard, page, PROT_NONE) }, 0, "mprotect failed");

    let mut rng = Rng::new(SEED ^ 4);
    for body_len in [0usize, 1, 2, 3, 7, 8, 15, 16, 17, 31, 32, 63, 64, 100, 1000, page - 1] {
        // Place body so that the terminator sits at the LAST byte of the page.
        let start = page - body_len - 1;
        let region = unsafe { std::slice::from_raw_parts_mut(base as *mut u8, page) };
        for i in 0..body_len {
            region[start + i] = rng.nonnul_byte();
        }
        region[page - 1] = 0;
        let expect: Vec<u8> = region[start..page].to_vec();

        let p = unsafe { (base as *const u8).add(start) } as *const c_char;
        let cp = unsafe { (l.c)(p) };
        let rp = unsafe { (l.rs)(p) };
        assert!(!cp.is_null() && !rp.is_null(), "err04 len={body_len}: unexpected NULL");
        let n = body_len + 1;
        let cout = unsafe { std::slice::from_raw_parts(cp as *const u8, n) }.to_vec();
        let rout = unsafe { std::slice::from_raw_parts(rp as *const u8, n) }.to_vec();
        assert_eq!(cout, rout, "err04 len={body_len}: C/Rust differ at the mapping edge");
        assert_eq!(cout, expect, "err04 len={body_len}: wrong bytes at the mapping edge");
        unsafe {
            common::free(cp as *mut _);
            common::free(rp as *mut _);
        }
    }
    unsafe { munmap(base, 2 * page) };
}

// ---------------------------------------------------------------------------
// G3 — oversized lengths
// ---------------------------------------------------------------------------

#[test]
fn g03_oversized_lengths() {
    for &len in &[1024 * 1024usize, 4 * 1024 * 1024] {
        let body = vec![b'Z'; len];
        let buf = CStrBuf::new(&body);
        let out = assert_same(&buf, &format!("g03 len={len}"));
        assert_eq!(out.len(), len + 1);
    }
}

// ---------------------------------------------------------------------------
// G4 — embedded NUL: bytes after the first NUL must NOT be copied.
// ---------------------------------------------------------------------------

#[test]
fn g04_embedded_nul_stops_copy() {
    let l = libs();
    let bytes = b"ab\0cd\0".to_vec();
    let p = bytes.as_ptr() as *const c_char;
    let cp = unsafe { (l.c)(p) };
    let rp = unsafe { (l.rs)(p) };
    assert!(!cp.is_null() && !rp.is_null());
    let cout = unsafe { std::slice::from_raw_parts(cp as *const u8, 3) }.to_vec();
    let rout = unsafe { std::slice::from_raw_parts(rp as *const u8, 3) }.to_vec();
    assert_eq!(cout, b"ab\0".to_vec(), "g04: C copied past the first NUL");
    assert_eq!(rout, cout, "g04: Rust differs from C");
    unsafe {
        common::free(cp as *mut _);
        common::free(rp as *mut _);
    }
}

// ---------------------------------------------------------------------------
// G5 — arbitrary binary bytes 0x01..0xFF before the terminator.
// ---------------------------------------------------------------------------

#[test]
fn g05_all_nonnul_bytes() {
    let body: Vec<u8> = (1u8..=255).collect();
    let buf = CStrBuf::new(&body);
    let out = assert_same(&buf, "g05 all non-NUL bytes");
    assert_eq!(&out[..255], &body[..]);
}

// ---------------------------------------------------------------------------
// G6 — out-of-range enum values across the FFI boundary.
//
// The API has no enum, flag, or integer parameter, so there is no discriminant
// to push out of range. What we CAN do is push the single pointer parameter to
// values no valid caller would pass and confirm both sides agree: NULL (the one
// value the C inspects) is rejected identically, and the C performs no other
// validation, so any non-NULL value is dereferenced by both. We therefore probe
// the full set of "special" pointer bit patterns the C actually distinguishes,
// which is exactly {0} vs {everything else}.
// ---------------------------------------------------------------------------

#[test]
fn g06_no_enum_param_pointer_domain_partition() {
    // The only value in the domain that the C treats specially is 0.
    assert_both_null(0usize as *const c_char, "g06 pointer value 0 (NULL)");

    // And every non-NULL value that points at a valid NUL-terminated buffer is
    // accepted identically — including pointers into read-only static memory,
    // stack memory, and heap memory (three different mappings).
    let l = libs();
    static STATIC_STR: &[u8] = b"static-rodata\0";
    let stack: [u8; 8] = *b"stackxy\0";
    let heap = CStrBuf::new(b"heap-allocated");

    for (ctx, p, n) in [
        ("rodata", STATIC_STR.as_ptr() as *const c_char, STATIC_STR.len()),
        ("stack", stack.as_ptr() as *const c_char, stack.len()),
        ("heap", heap.ptr(), heap.expected_copy().len()),
    ] {
        let cp = unsafe { (l.c)(p) };
        let rp = unsafe { (l.rs)(p) };
        assert!(!cp.is_null() && !rp.is_null(), "g06 {ctx}: unexpected NULL");
        let cout = unsafe { std::slice::from_raw_parts(cp as *const u8, n) }.to_vec();
        let rout = unsafe { std::slice::from_raw_parts(rp as *const u8, n) }.to_vec();
        assert_eq!(cout, rout, "g06 {ctx}: C/Rust differ");
        unsafe {
            common::free(cp as *mut _);
            common::free(rp as *mut _);
        }
    }
}

// ---------------------------------------------------------------------------
// G7 — misaligned pointer (covered at volume in Phase B row 10; the strict
// one-step-past-each-alignment sweep lives here).
// ---------------------------------------------------------------------------

#[test]
fn g07_every_alignment_offset() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 7);
    for off in 0..64usize {
        for len in 0..40usize {
            let mut backing: Vec<u8> = (0..off).map(|_| rng.nonnul_byte()).collect();
            let body: Vec<u8> = (0..len).map(|_| rng.nonnul_byte()).collect();
            backing.extend_from_slice(&body);
            backing.push(0);
            let p = unsafe { backing.as_ptr().add(off) } as *const c_char;
            let cp = unsafe { (l.c)(p) };
            let rp = unsafe { (l.rs)(p) };
            assert!(!cp.is_null() && !rp.is_null(), "g07 off={off} len={len}");
            let n = len + 1;
            let cout = unsafe { std::slice::from_raw_parts(cp as *const u8, n) }.to_vec();
            let rout = unsafe { std::slice::from_raw_parts(rp as *const u8, n) }.to_vec();
            assert_eq!(cout, rout, "g07 off={off} len={len}: C/Rust differ");
            assert_eq!(&cout[..len], &body[..]);
            unsafe {
                common::free(cp as *mut _);
                common::free(rp as *mut _);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// G8 — returned pointer is free()-able by the caller for BOTH libraries.
// A Rust translation that used the Rust global allocator instead of libc malloc
// would abort or corrupt the heap here.
// ---------------------------------------------------------------------------

#[test]
fn g08_returned_pointer_is_libc_freeable() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..3000 {
        let len = rng.range(0, 2000);
        let body: Vec<u8> = (0..len).map(|_| rng.nonnul_byte()).collect();
        let buf = CStrBuf::new(&body);
        let cp = unsafe { (l.c)(buf.ptr()) };
        let rp = unsafe { (l.rs)(buf.ptr()) };
        assert!(!cp.is_null() && !rp.is_null(), "g08 i={i}");
        // free through libc, exactly as a C consumer of either .so would.
        unsafe {
            common::free(cp as *mut _);
            common::free(rp as *mut _);
        }
    }
    // Reaching here without an allocator abort is the assertion.
    let _ = l;
}
