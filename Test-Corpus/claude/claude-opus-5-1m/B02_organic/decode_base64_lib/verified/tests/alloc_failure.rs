//! Phase C — the allocation-failure rejection branches of the C
//! (`ERRORS.md` rows 3, 3b, 3c and 4).
//!
//! * row 3  — `calloc` fails                        -> `return NULL`
//! * row 3b — `int l = strlen(src) + 1` overflows so `l + 13` converts to a
//!            huge `size_t` and `calloc` fails       -> `return NULL`
//! * row 4  — `calloc` succeeds but `malloc` fails   -> `free(dest); return NULL`
//!
//! Two independent mechanisms are used, because a differential test of an
//! allocation-failure branch is only worth anything if it *proves* which
//! allocation failed:
//!
//! 1. **Deterministic, from the C's own integer arithmetic** (no environment
//!    dependence at all): `l` is an `int`, so for `strlen(src) == 4294967282`
//!    the expression `strlen(src) + 1` truncates to `l == -13`. Then
//!    `calloc(sizeof(char), l + 13)` is `calloc(1, 0)` — which *succeeds* — and
//!    the following `malloc(l)` converts `-13` to `(size_t)0xFFFF…F3` — which
//!    *always* fails. That is exactly row 4, hit without touching any limit.
//!    Neighbouring values of `strlen` give the row 3 (`calloc` fails) variant.
//! 2. **`RLIMIT_AS` + ballast**, as a second, independent reproduction of the
//!    same two branches at realistic sizes. The child exhausts the address
//!    space (a plain small `RLIMIT_AS` is *not* enough — glibc happily reuses
//!    existing free chunks, and a failing `malloc` does not even imply the next
//!    one fails), then *probe* allocations prove which of the two allocations
//!    the resulting state starves before `decode_base64` is called. If the
//!    intended window cannot be created the child reports "inconclusive" and
//!    another strategy is tried, so the test can never pass vacuously.

mod common;

use common::DecodeFn;
use std::ffi::{c_char, c_int, c_void};
use std::hint::black_box;

const RLIMIT_AS: c_int = 9; // Linux
const WNOHANG: c_int = 1;

#[repr(C)]
struct RLimit {
    cur: u64,
    max: u64,
}

unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut c_int, options: c_int) -> i32;
    fn kill(pid: i32, sig: c_int) -> c_int;
    fn getrlimit(resource: c_int, rlim: *mut RLimit) -> c_int;
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
    fn _exit(code: c_int) -> !;
    fn calloc(n: usize, sz: usize) -> *mut c_void;
    fn malloc(sz: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

fn vsize_bytes() -> u64 {
    let s = std::fs::read_to_string("/proc/self/statm").expect("read statm");
    let pages: u64 = s
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .expect("parse statm");
    pages * 4096
}

fn mem_available_bytes() -> u64 {
    let s = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            if let Some(kb) = rest.split_whitespace().next() {
                if let Ok(v) = kb.parse::<u64>() {
                    return v * 1024;
                }
            }
        }
    }
    0
}

// ===========================================================================
// 2. independent reproduction with RLIMIT_AS + address-space exhaustion
// ===========================================================================

/// Which allocation inside `decode_base64` the budget must starve.
#[derive(Copy, Clone, PartialEq, Debug)]
enum Expect {
    /// `calloc(1, l + 13)` itself fails (row 3)
    CallocFails,
    /// `calloc` succeeds, the following `malloc(l)` fails (row 4)
    MallocFails,
}

// child exit codes
const OK: c_int = 0;
const BOTH_SUCCEEDED: c_int = 10;
const RUST_NOT_NULL: c_int = 11;
const RUST_NULL_ONLY: c_int = 12;
const SETRLIMIT_FAILED: c_int = 20;
const PROBE_CALLOC_FAILED: c_int = 21;
const PROBE_MALLOC_SUCCEEDED: c_int = 22;
const PROBE_CALLOC_SUCCEEDED: c_int = 23;
const NO_BALLAST: c_int = 24;
const DRAIN_INCOMPLETE: c_int = 25;

/// A ballast strategy: address-space budget in multiples of `cap`, plus which
/// tracked ballast block to release (row 4 only).
#[derive(Copy, Clone, Debug)]
struct Strategy {
    caps: u64,
    free_which: u8, // 0 = middle, 1 = first, 2 = last
}

/// Runs one attempt in a forked child; returns the child's exit code.
#[allow(clippy::too_many_arguments)]
fn run_child(
    f_c: DecodeFn,
    f_r: DecodeFn,
    src: *const c_char,
    cap: usize,
    l: usize,
    vsize: u64,
    expect: Expect,
    st: Strategy,
) -> i32 {
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        // ------- child: raw libc only, no Rust allocation, no panicking -------
        unsafe {
            let mut orig = RLimit { cur: 0, max: 0 };
            if getrlimit(RLIMIT_AS, &mut orig) != 0 {
                _exit(SETRLIMIT_FAILED);
            }
            let rl = RLimit {
                cur: vsize + st.caps * cap as u64,
                max: orig.max, // keep the hard limit, only squeeze the soft one
            };
            if setrlimit(RLIMIT_AS, &rl) != 0 {
                _exit(SETRLIMIT_FAILED);
            }

            // ---- ballast phase 1: destination-sized blocks, tracked ----
            // The list is threaded through the blocks themselves so that no
            // additional allocation is needed to remember them.
            let mut head: *mut c_void = std::ptr::null_mut();
            let mut count: usize = 0;
            loop {
                // `black_box` is mandatory here: LLVM recognises `malloc` as an
                // allocation function and happily DELETES a call whose result is
                // otherwise unused, folding the null check to "not null" — an
                // optimized test binary would then "allocate" terabytes without
                // the address space ever growing, and the starvation would be
                // fake. Forcing the pointer (and the size) to escape keeps every
                // allocation real in every profile.
                let p = black_box(malloc(black_box(cap)));
                if p.is_null() {
                    break;
                }
                *(p as *mut *mut c_void) = head;
                head = p;
                count += 1;
                if count > 64 {
                    break; // safety valve
                }
            }

            // ---- ballast phase 2: drain everything else, untracked ----
            // A failing `malloc(cap)` does NOT mean the allocator is out of
            // memory: it may still hold reusable chunks (and other arenas) that
            // a later request of the very same size can be served from. Keep
            // halving the request size until even a page-sized block fails, so
            // the starvation is real.
            // Every successful allocation consumes at least one page of the
            // address space (either fresh or from a reusable hole), and the
            // total address space can never exceed the soft limit, so
            // `limit / 4096` is a hard upper bound on the number of allocations
            // the drain loop can possibly make. The guard can therefore never
            // trip early — and if it ever did, the child reports it instead of
            // continuing with an unproven state.
            let max_blocks = (vsize + st.caps * cap as u64) / 4096 + 8192;
            let mut size = cap / 2;
            let mut guard: u64 = 0;
            'drain: while size >= 4096 {
                loop {
                    let p = black_box(malloc(black_box(size)));
                    if p.is_null() {
                        break;
                    }
                    guard += 1;
                    if guard > max_blocks {
                        break 'drain;
                    }
                }
                size /= 2;
            }
            if guard > max_blocks {
                _exit(DRAIN_INCOMPLETE);
            }

            if expect == Expect::MallocFails {
                if count == 0 {
                    _exit(NO_BALLAST);
                }
                // Release exactly one destination-sized block: from here on
                // precisely one such allocation can be served, and nothing more.
                let idx = match st.free_which {
                    1 => 0,
                    2 => count - 1,
                    _ => count / 2,
                };
                let mut prev: *mut c_void = std::ptr::null_mut();
                let mut cur = head;
                let mut i = 0usize;
                while i < idx {
                    prev = cur;
                    cur = *(cur as *mut *mut c_void);
                    i += 1;
                }
                let next = *(cur as *mut *mut c_void);
                if prev.is_null() {
                    head = next;
                } else {
                    *(prev as *mut *mut c_void) = next;
                }
                free(cur);
            }

            // ---- probes: prove the intended allocation is the failing one ----
            let p1 = black_box(calloc(black_box(1), black_box(cap)));
            match expect {
                Expect::CallocFails => {
                    if !p1.is_null() {
                        free(p1);
                        _exit(PROBE_CALLOC_SUCCEEDED);
                    }
                }
                Expect::MallocFails => {
                    if p1.is_null() {
                        _exit(PROBE_CALLOC_FAILED);
                    }
                    let p2 = black_box(malloc(black_box(l)));
                    if !p2.is_null() {
                        free(p2);
                        free(p1);
                        _exit(PROBE_MALLOC_SUCCEEDED);
                    }
                    free(p1);
                }
            }

            // ---------------- the actual differential call ----------------
            let a = f_c(src);
            let b = f_r(src);
            let an = a.is_null();
            let bn = b.is_null();
            if !an {
                free(a as *mut c_void);
            }
            if !bn {
                free(b as *mut c_void);
            }
            let _ = head; // the ballast is released by process exit
            match (an, bn) {
                (true, true) => _exit(OK),
                (true, false) => _exit(RUST_NOT_NULL),
                (false, true) => _exit(RUST_NULL_ONLY),
                (false, false) => _exit(BOTH_SUCCEEDED),
            }
        }
    }
    // ------------------------------- parent -------------------------------
    let mut status: c_int = 0;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    loop {
        let r = unsafe { waitpid(pid, &mut status, WNOHANG) };
        if r == pid {
            break;
        }
        assert!(r == 0, "waitpid failed: {r}");
        if std::time::Instant::now() > deadline {
            unsafe { kill(pid, 9) };
            let _ = unsafe { waitpid(pid, &mut status, 0) };
            panic!("child hung for 120s under RLIMIT_AS");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(
        status & 0x7f == 0,
        "child was killed by signal {} (status 0x{status:x}) — crashing under \
         allocation failure is itself a divergence from the C",
        status & 0x7f
    );
    (status >> 8) & 0xff
}

/// `true` if the attempt verified the row, `false` if it was inconclusive.
fn interpret(code: i32, what: &str) -> bool {
    match code {
        OK => true,
        RUST_NOT_NULL => {
            panic!("{what}: DIVERGENCE — C returned NULL, Rust returned a non-NULL pointer")
        }
        RUST_NULL_ONLY => {
            panic!("{what}: DIVERGENCE — C returned a non-NULL pointer, Rust returned NULL")
        }
        BOTH_SUCCEEDED | PROBE_CALLOC_FAILED | PROBE_MALLOC_SUCCEEDED | PROBE_CALLOC_SUCCEEDED
        | NO_BALLAST | DRAIN_INCOMPLETE => false,
        SETRLIMIT_FAILED => panic!("{what}: getrlimit/setrlimit(RLIMIT_AS) failed in the child"),
        other => panic!("{what}: unexpected child exit code {other}"),
    }
}

fn verify_row(label: &str, input: &[u8], expect: Expect) {
    let a = common::api();
    let src = input.as_ptr() as *const c_char;
    let l = input.len(); // == strlen(src) + 1
    let cap = l + 13; // == calloc(1, l + 13)
    let mut last = -1;
    for caps in [4u64, 2, 8, 1, 16] {
        for free_which in [0u8, 1, 2] {
            let st = Strategy { caps, free_which };
            let vsize = vsize_bytes();
            let code = run_child(a.c, a.rust, src, cap, l, vsize, expect, st);
            last = code;
            if interpret(code, label) {
                eprintln!("{label}: VERIFIED (strategy {st:?}, {expect:?})");
                return;
            }
            eprintln!("{label}: inconclusive (child code {code}, strategy {st:?}), retrying");
            if expect == Expect::CallocFails {
                break; // free_which is irrelevant for this row
            }
        }
    }
    panic!(
        "{label}: could not create the required allocation window \
         (last child exit code {last}); the branch was NOT verified"
    );
}

/// 64 MiB of valid base64 — `cap` is then always above glibc's maximum mmap
/// threshold (32 MiB), so the blocks involved get their own mappings and the
/// address-space accounting is exact.
fn big_input() -> Vec<u8> {
    let n = 64 * 1024 * 1024usize;
    assert!(
        mem_available_bytes() > 512 * 1024 * 1024,
        "not enough free memory for the allocation-failure tests"
    );
    let mut v = vec![b'A'; n];
    v.push(0);
    v
}

/// ERRORS.md row 3, second mechanism — `calloc(sizeof(char), l + 13)` returns
/// NULL at a realistic size because the address space is exhausted.
#[test]
fn e3_calloc_fails_under_rlimit() {
    let input = big_input();
    verify_row("ERRORS row 3 (calloc fails, RLIMIT_AS)", &input, Expect::CallocFails);
}

/// ERRORS.md row 4, second mechanism — `calloc` succeeds, `malloc(l)` returns
/// NULL, so the C frees `dest` and returns NULL.
#[test]
fn e4_malloc_fails_under_rlimit() {
    let input = big_input();
    verify_row(
        "ERRORS row 4 (malloc fails after calloc, RLIMIT_AS)",
        &input,
        Expect::MallocFails,
    );
}

