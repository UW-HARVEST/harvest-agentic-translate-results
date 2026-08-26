//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! The C library signals failure only by returning `NULL` (or by faulting on an
//! unchecked pointer). Both kinds of behaviour are compared between the C `.so`
//! and the Rust `.so`:
//!
//! * allocation failures are produced deterministically by `fork()`ing a child,
//!   lowering `RLIMIT_AS` to "current VM size + slack" and shaping the input so
//!   that exactly the targeted `malloc`/`realloc`/`strdup` exceeds the slack;
//! * faults / non-termination are compared through the child's wait status.
//!
//! Each implementation runs in its own freshly forked child, so both see an
//! identical memory state.

#![allow(non_snake_case)]

mod harness;

use harness::*;
use std::ffi::c_char;
use std::sync::{Mutex, MutexGuard};

/// The fork+RLIMIT_AS tests measure the process' address-space usage, so only
/// one of them may run at a time (cargo runs tests in parallel by default).
fn serial() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::new(());
    M.lock().unwrap_or_else(|e| e.into_inner())
}

/// Build a NUL-terminated buffer of `len` copies of `b` without ever creating
/// a large free chunk in the parent's heap (exact capacity, no realloc).
fn filled_z(b: u8, len: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(len + 1);
    v.resize(len, b);
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// child-process plumbing
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Status {
    Exited(i32),
    Signaled(i32),
}

/// Run `child` in a forked process and report how it terminated.
/// The closure must not panic and must not allocate through Rust's allocator.
fn fork_run(child: impl FnOnce() -> i32) -> Status {
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            let code = child();
            libc::_exit(code);
        }
        let mut st: i32 = 0;
        let w = libc::waitpid(pid, &mut st, 0);
        assert_eq!(w, pid, "waitpid failed");
        if libc::WIFEXITED(st) {
            Status::Exited(libc::WEXITSTATUS(st))
        } else if libc::WIFSIGNALED(st) {
            Status::Signaled(libc::WTERMSIG(st))
        } else {
            panic!("child neither exited nor was signalled: raw status {st}")
        }
    }
}

fn page_size() -> u64 {
    unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 }
}

/// Current virtual-memory size of this process (first field of /proc/self/statm,
/// in pages). Read in the PARENT; the child inherits exactly this size.
fn vm_size() -> u64 {
    let s = std::fs::read_to_string("/proc/self/statm").expect("read /proc/self/statm");
    let pages: u64 = s
        .split_whitespace()
        .next()
        .expect("statm field")
        .parse()
        .expect("statm number");
    pages * page_size()
}

/// Exit codes used by the children (kept distinct from 0/1 so that a setup
/// failure can never masquerade as "returned NULL").
const EXIT_NULL: i32 = 0; // implementation returned NULL
const EXIT_NON_NULL: i32 = 1; // implementation returned a pointer
const EXIT_SETRLIMIT_FAILED: i32 = 90;

/// Child body: cap the address space, then make the single call.
fn capped_call(f: SarFn, o: &[u8], s: &[u8], v: &[u8], limit: u64, alarm_s: u32) -> i32 {
    unsafe {
        let rl = libc::rlimit { rlim_cur: limit, rlim_max: limit };
        if libc::setrlimit(libc::RLIMIT_AS, &rl) != 0 {
            return EXIT_SETRLIMIT_FAILED;
        }
        if alarm_s > 0 {
            libc::alarm(alarm_s);
        }
        let p = f(
            o.as_ptr() as *const c_char,
            s.as_ptr() as *const c_char,
            v.as_ptr() as *const c_char,
        );
        if p.is_null() { EXIT_NULL } else { EXIT_NON_NULL }
    }
}

/// Differentially exercise an allocation-failure row.
/// `slack` is how much MORE address space the child may use beyond what the
/// process already occupies; the input must be shaped so that the targeted
/// allocation is larger than `slack`.
fn diff_alloc_failure(row: &str, orig: &[u8], search: &[u8], value: &[u8], slack: u64) {
    diff_alloc_failure_z(row, &cstr(orig), &cstr(search), &cstr(value), slack)
}

/// Same as [`diff_alloc_failure`] but the three buffers are ALREADY
/// NUL-terminated — used for the multi-hundred-megabyte inputs, where copying
/// them again in the parent would double the memory footprint.
fn diff_alloc_failure_z(row: &str, o: &[u8], s: &[u8], v: &[u8], slack: u64) {
    assert_eq!(o.last(), Some(&0), "buffer must be NUL-terminated");
    assert_eq!(s.last(), Some(&0), "buffer must be NUL-terminated");
    assert_eq!(v.last(), Some(&0), "buffer must be NUL-terminated");
    let _g = serial();
    let (cf, rf) = fns(); // load both .so BEFORE measuring the VM size
    let limit = vm_size() + slack;

    let c = fork_run(|| capped_call(cf, &o, &s, &v, limit, 120));
    let r = fork_run(|| capped_call(rf, &o, &s, &v, limit, 120));

    assert_ne!(
        c,
        Status::Exited(EXIT_SETRLIMIT_FAILED),
        "[{row}] setrlimit failed in the child — test setup problem"
    );
    assert_eq!(c, r, "[{row}] C and Rust disagree: C={c:?} Rust={r:?}");
    assert_eq!(
        c,
        Status::Exited(EXIT_NULL),
        "[{row}] expected BOTH to return NULL on allocation failure, got {c:?}"
    );
}

/// Differentially exercise a faulting / non-terminating row.
fn diff_fault(
    row: &str,
    o: *const c_char,
    s: *const c_char,
    v: *const c_char,
    alarm_s: u32,
    expect: Status,
) {
    let _g = serial();
    let (cf, rf) = fns();
    let call_it = move |f: SarFn| {
        move || unsafe {
            if alarm_s > 0 {
                libc::alarm(alarm_s);
            }
            let p = f(o, s, v);
            if p.is_null() { EXIT_NULL } else { EXIT_NON_NULL }
        }
    };
    let c = fork_run(call_it(cf));
    let r = fork_run(call_it(rf));
    assert_eq!(c, r, "[{row}] C and Rust disagree: C={c:?} Rust={r:?}");
    assert_eq!(c, expect, "[{row}] unexpected outcome {c:?} (expected {expect:?})");
}

/// The targeted allocation must be larger than glibc's maximum dynamic mmap
/// threshold (`DEFAULT_MMAP_THRESHOLD_MAX` = 32 MiB on 64-bit) so that malloc
/// MUST create a new mapping — otherwise the request can be served from the
/// thread arena's pre-reserved (already mapped, 64 MiB) address space and
/// RLIMIT_AS would not stop it.
const BIG: usize = 128 * 1024 * 1024; // 128 MiB
const SLACK: u64 = 8 * 1024 * 1024; //    8 MiB

// ---------------------------------------------------------------------------
// ERRORS.md row 1 — no match at all: returns a fresh copy of `orig`
// ---------------------------------------------------------------------------
#[test]
fn err01_no_match_returns_copy() {
    let mut rng = Rng::new(101);
    for _ in 0..2000 {
        let orig = rng.bytes_b(48, FILL);
        let search = rng.bytes_r(1, 5, SEARCH); // never occurs in FILL bytes
        let value = rng.bytes_b(6, VALUE);
        let out = check("err01", &orig, &search, &value);
        assert!(!out.null, "no-match path must not return NULL");
        assert_eq!(out.bytes, orig);
    }
    // search longer than orig, and orig empty
    check("err01b", b"ab", b"abc", b"z");
    check("err01c", b"", b"a", b"z");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 — strdup() failure on the no-match path
// ---------------------------------------------------------------------------
#[test]
fn err02_strdup_failure_returns_null() {
    let orig = filled_z(b'N', BIG);
    diff_alloc_failure_z("err02", &orig, b"ZZZ\0", b"v\0", SLACK);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 3 — malloc() failure for the prefix before the first match
// ---------------------------------------------------------------------------
#[test]
fn err03_prefix_malloc_failure_returns_null() {
    let mut orig = Vec::with_capacity(BIG + 3);
    orig.resize(BIG, b'P');
    orig.extend_from_slice(b"QQ\0"); // first match at offset BIG (> 0)
    diff_alloc_failure_z("err03", &orig, b"QQ\0", b"v\0", SLACK);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 4 — realloc() failure while copying the replacement value
// ---------------------------------------------------------------------------
#[test]
fn err04_value_realloc_failure_returns_null() {
    let value = filled_z(b'V', BIG);
    // small orig with one match at offset 2 -> the prefix malloc(3) succeeds,
    // then realloc(3 -> 3 + 128 MiB) must fail
    diff_alloc_failure_z("err04", b"abQQcd\0", b"QQ\0", &value, SLACK);
    // and the same with the match at offset 0 (realloc from NULL)
    diff_alloc_failure_z("err04b", b"QQcd\0", b"QQ\0", &value, SLACK);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 5 — realloc() failure while copying the gap between matches
// ---------------------------------------------------------------------------
#[test]
fn err05_gap_realloc_failure_returns_null() {
    let mut orig = Vec::with_capacity(BIG + 3);
    orig.push(b'Q'); // match #1 at offset 0
    orig.resize(BIG + 1, b'G'); // 128 MiB gap
    orig.push(b'Q'); // match #2
    orig.push(0);
    diff_alloc_failure_z("err05", &orig, b"Q\0", b"v\0", SLACK);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 6 — realloc() failure while copying the tail
// ---------------------------------------------------------------------------
#[test]
fn err06_tail_realloc_failure_returns_null() {
    let mut orig = Vec::with_capacity(BIG + 2);
    orig.push(b'Q'); // single match at offset 0
    orig.resize(BIG + 1, b'T'); // 128 MiB tail
    orig.push(0);
    diff_alloc_failure_z("err06", &orig, b"Q\0", b"v\0", SLACK);
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 7/8/9 — NULL arguments (unchecked strlen -> SIGSEGV)
// ---------------------------------------------------------------------------
#[test]
fn err07_null_orig_faults() {
    let s = cstr(b"X");
    let v = cstr(b"y");
    diff_fault(
        "err07",
        std::ptr::null(),
        s.as_ptr() as *const c_char,
        v.as_ptr() as *const c_char,
        10,
        Status::Signaled(libc::SIGSEGV),
    );
}

#[test]
fn err08_null_search_faults() {
    let o = cstr(b"aXb");
    let v = cstr(b"y");
    diff_fault(
        "err08",
        o.as_ptr() as *const c_char,
        std::ptr::null(),
        v.as_ptr() as *const c_char,
        10,
        Status::Signaled(libc::SIGSEGV),
    );
}

#[test]
fn err09_null_value_faults() {
    // `strlen(value)` happens BEFORE the strstr early-out, so this must fault
    // even though `search` does occur / does not occur in `orig`.
    let o = cstr(b"aXb");
    let s = cstr(b"X");
    diff_fault(
        "err09",
        o.as_ptr() as *const c_char,
        s.as_ptr() as *const c_char,
        std::ptr::null(),
        10,
        Status::Signaled(libc::SIGSEGV),
    );
    // no-match variant: value is still dereferenced first
    let s2 = cstr(b"Z");
    diff_fault(
        "err09b",
        o.as_ptr() as *const c_char,
        s2.as_ptr() as *const c_char,
        std::ptr::null(),
        10,
        Status::Signaled(libc::SIGSEGV),
    );
}

#[test]
fn err07_09_all_null_faults() {
    diff_fault(
        "err07-09-all-null",
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
        10,
        Status::Signaled(libc::SIGSEGV),
    );
}

// ---------------------------------------------------------------------------
// ERRORS.md row 10 — empty search + empty value: the loop cannot terminate
// ---------------------------------------------------------------------------
#[test]
fn err10_empty_search_empty_value_never_returns() {
    let o = cstr(b"abc");
    let s = cstr(b"");
    let v = cstr(b"");
    diff_fault(
        "err10",
        o.as_ptr() as *const c_char,
        s.as_ptr() as *const c_char,
        v.as_ptr() as *const c_char,
        2,
        Status::Signaled(libc::SIGALRM),
    );
    // also with an empty orig
    let o2 = cstr(b"");
    diff_fault(
        "err10b",
        o2.as_ptr() as *const c_char,
        s.as_ptr() as *const c_char,
        v.as_ptr() as *const c_char,
        2,
        Status::Signaled(libc::SIGALRM),
    );
}

// ---------------------------------------------------------------------------
// ERRORS.md row 11 — empty search + non-empty value: unbounded growth until
// realloc() fails, then NULL
// ---------------------------------------------------------------------------
#[test]
fn err11_empty_search_nonempty_value_exhausts_memory() {
    let value = vec![b'y'; 4096];
    diff_alloc_failure("err11", b"x", b"", &value, SLACK);
    diff_alloc_failure("err11b", b"", b"", &value, SLACK);
    // longer orig / 1-byte-at-a-time growth is the same code path but far
    // slower, so it is exercised with a bigger value only.
    let value2 = vec![b'z'; 64 * 1024];
    diff_alloc_failure("err11c", b"abc", b"", &value2, SLACK);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 12 — no enum / flag / integer parameter exists in this API,
// so there is no out-of-range-enum input. Documented by construction: the
// header declares exactly three `const char *` parameters.
// ---------------------------------------------------------------------------
#[test]
fn err12_no_enum_parameters_in_api() {
    let hdr = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/include/lib.h"))
        .expect("read lib.h");
    assert!(!hdr.contains("enum"), "an enum appeared in the public API: {hdr}");
    assert!(
        hdr.contains("char *searchAndReplace(const char *orig, const char *search, const char *value);"),
        "unexpected public API: {hdr}"
    );
}

// ---------------------------------------------------------------------------
// Generic FFI boundary cases beyond the table
// ---------------------------------------------------------------------------
#[test]
fn errX_zero_length_inputs() {
    // every combination of empty/non-empty for the three arguments, except the
    // empty-`search` ones (rows 10/11 — they do not terminate)
    for &o in [&b""[..], &b"a"[..], &b"aXbXc"[..]].iter() {
        for &s in [&b"X"[..], &b"aXb"[..]].iter() {
            for &v in [&b""[..], &b"z"[..], &b"zzzz"[..]].iter() {
                check("errX-zero", o, s, v);
            }
        }
    }
}

#[test]
fn errX_aliased_pointers() {
    // the same buffer passed as several arguments (legal: all are const)
    let (cf, rf) = fns();
    for buf in [&b"ab"[..], &b"X"[..], &b"abcabc"[..]] {
        let b = cstr(buf);
        let p = b.as_ptr() as *const c_char;
        let c = unsafe { call_raw(cf, p, p, p) };
        let r = unsafe { call_raw(rf, p, p, p) };
        assert_eq!(c, r, "aliased orig==search==value diverged for {buf:?}");
        // orig aliased with value only
        let s = cstr(b"b");
        let sp = s.as_ptr() as *const c_char;
        let c2 = unsafe { call_raw(cf, p, sp, p) };
        let r2 = unsafe { call_raw(rf, p, sp, p) };
        assert_eq!(c2, r2, "aliased orig==value diverged for {buf:?}");
    }
}

#[test]
fn errX_one_past_valid_search_length() {
    // "one step past the valid range": a search string exactly one byte longer
    // than orig can never match, whatever the bytes are.
    let mut rng = Rng::new(112);
    for _ in 0..500 {
        // `orig` is kept non-empty so that `search == orig` never becomes the
        // empty needle (ERRORS.md rows 10/11 — that path never returns).
        let orig = rng.bytes_r(1, 24, b"ab");
        let mut search = orig.clone();
        search.push(b'a');
        let out = check("errX-one-past", &orig, &search, b"Z");
        assert_eq!(out.bytes, orig);
        // and exactly-equal length (the boundary that CAN match)
        check("errX-boundary", &orig, &orig.clone(), b"Z");
        // empty orig with a one-byte search: the other side of the boundary
        let out2 = check("errX-empty-orig", b"", &orig, b"Z");
        assert!(out2.bytes.is_empty());
    }
}
