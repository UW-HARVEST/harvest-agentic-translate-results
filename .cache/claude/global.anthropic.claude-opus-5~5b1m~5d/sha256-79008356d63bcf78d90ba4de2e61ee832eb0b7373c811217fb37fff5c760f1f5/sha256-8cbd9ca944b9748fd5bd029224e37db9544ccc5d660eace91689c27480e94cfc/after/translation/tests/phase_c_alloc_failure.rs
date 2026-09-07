//! Phase C, ERRORS.md row 3 — the `malloc` failure path (lib.c:64-67).
//!
//! ```c
//! number_c_string = (unsigned char *) malloc(number_string_length + 1);
//! if (number_c_string == NULL)
//! {
//!     return false; /* allocation failure */
//! }
//! ```
//!
//! To reach it we need `malloc(number_string_length + 1)` to actually fail,
//! which requires BOTH a very large `number_string_length` and a tight address
//! space limit. We get there without burning real memory by:
//!
//!  1. creating a small anonymous shared file (`memfd_create`) filled with
//!     `'1'` bytes, and
//!  2. mapping that same page range `MAP_FIXED` many times back-to-back, so a
//!     large *virtual* run of digits is backed by a tiny amount of *physical*
//!     memory, then
//!  3. `fork()`ing and lowering `RLIMIT_AS` in the child so the library's
//!     `malloc` of that size is guaranteed to return NULL.
//!
//! The child calls the C `.so` and then the Rust `.so` and reports both return
//! values through its exit status, so this is a true differential test.

mod common;

use common::*;
use std::ffi::c_void;

const CHUNK: usize = 1 << 20; // 1 MiB memfd, mapped repeatedly
const REPEATS: usize = 256; // => 256 MiB virtual run of '1'
const RUN_LEN: usize = CHUNK * REPEATS;

const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_SHARED: i32 = 0x01;
const MAP_PRIVATE: i32 = 0x02;
const MAP_FIXED: i32 = 0x10;
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_NORESERVE: i32 = 0x4000;
const MAP_FAILED: *mut c_void = usize::MAX as *mut c_void;
const RLIMIT_AS: i32 = 9;

#[repr(C)]
struct RLimit {
    rlim_cur: u64,
    rlim_max: u64,
}

unsafe extern "C" {
    fn memfd_create(name: *const i8, flags: u32) -> i32;
    fn ftruncate(fd: i32, len: i64) -> i32;
    fn pwrite(fd: i32, buf: *const c_void, n: usize, off: i64) -> isize;
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut c_void;
    fn munmap(addr: *mut c_void, len: usize) -> i32;
    fn fork() -> i32;
    fn setrlimit(res: i32, lim: *const RLimit) -> i32;
    fn getrlimit(res: i32, lim: *mut RLimit) -> i32;
    fn waitpid(pid: i32, status: *mut i32, opts: i32) -> i32;
    fn _exit(code: i32) -> !;
}

/// Reserve `RUN_LEN + slack` of address space, then overwrite the first
/// `RUN_LEN` bytes with repeated MAP_FIXED mappings of the same memfd page
/// range. Returns the base pointer.
fn build_huge_digit_run() -> Option<*mut u8> {
    unsafe {
        let fd = memfd_create(b"digits\0".as_ptr() as *const i8, 0);
        if fd < 0 {
            return None;
        }
        if ftruncate(fd, CHUNK as i64) != 0 {
            return None;
        }
        // fill the memfd with '1'
        let ones = vec![b'1'; 64 * 1024];
        let mut off = 0usize;
        while off < CHUNK {
            let n = ones.len().min(CHUNK - off);
            if pwrite(fd, ones.as_ptr() as *const c_void, n, off as i64) != n as isize {
                return None;
            }
            off += n;
        }

        // reserve the whole span (plus one page of non-digit bytes as a stopper)
        let span = RUN_LEN + 4096;
        let base = mmap(
            std::ptr::null_mut(),
            span,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE,
            -1,
            0,
        );
        if base == MAP_FAILED {
            return None;
        }
        // the trailing page stays anonymous zero => '\0' terminates the scan
        for i in 0..REPEATS {
            let addr = (base as usize + i * CHUNK) as *mut c_void;
            let got = mmap(
                addr,
                CHUNK,
                PROT_READ,
                MAP_SHARED | MAP_FIXED | MAP_NORESERVE,
                fd,
                0,
            );
            if got == MAP_FAILED {
                munmap(base, span);
                return None;
            }
        }
        Some(base as *mut u8)
    }
}

#[test]
fn err_malloc_failure_returns_false() {
    let base = match build_huge_digit_run() {
        Some(p) => p,
        None => {
            eprintln!(
                "[row 3] SKIPPED: could not reserve {} MiB of address space for the \
                 huge-digit-run setup",
                RUN_LEN >> 20
            );
            return;
        }
    };

    // Sanity: the run really is `RUN_LEN` digits followed by a NUL page.
    unsafe {
        assert_eq!(*base, b'1');
        assert_eq!(*base.add(RUN_LEN - 1), b'1');
        assert_eq!(*base.add(RUN_LEN), 0, "stopper byte must not be accepted");
    }

    let p = pair();

    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            // ---- child ----
            // Cap the address space just above what we already occupy, so a
            // 256 MiB malloc inside the library cannot possibly succeed.
            let mut cur = RLimit { rlim_cur: 0, rlim_max: 0 };
            getrlimit(RLIMIT_AS, &mut cur);
            // current mappings are >= RUN_LEN; allow only 16 MiB more.
            let cap = (RUN_LEN as u64) + (64 << 20);
            let lim = RLimit {
                rlim_cur: cap.min(cur.rlim_max),
                rlim_max: cur.rlim_max,
            };
            if setrlimit(RLIMIT_AS, &lim) != 0 {
                _exit(70); // could not apply the limit -> inconclusive
            }

            let mut c_item = CJson::garbage();
            let mut c_buf = ParseBuffer {
                content: base,
                length: RUN_LEN,
                offset: 0,
                depth: 0,
            };
            let c_ret = p.c.parse_number_raw(&mut c_item, &mut c_buf);

            let mut r_item = CJson::garbage();
            let mut r_buf = ParseBuffer {
                content: base,
                length: RUN_LEN,
                offset: 0,
                depth: 0,
            };
            let r_ret = p.rust.parse_number_raw(&mut r_item, &mut r_buf);

            // encode: bit0 = C returned true, bit1 = Rust returned true,
            // bit2 = C mutated offset, bit3 = Rust mutated offset
            let mut code = 0;
            if c_ret != 0 {
                code |= 1;
            }
            if r_ret != 0 {
                code |= 2;
            }
            if c_buf.offset != 0 {
                code |= 4;
            }
            if r_buf.offset != 0 {
                code |= 8;
            }
            _exit(code);
        }

        // ---- parent ----
        let mut status = 0i32;
        assert_eq!(waitpid(pid, &mut status, 0), pid, "waitpid failed");
        munmap(base as *mut c_void, RUN_LEN + 4096);

        let exited = (status & 0x7f) == 0;
        let code = (status >> 8) & 0xff;
        assert!(
            exited,
            "child died on a signal (status {status:#x}); the allocation-failure \
             path must not crash either implementation"
        );

        if code == 70 {
            eprintln!("[row 3] SKIPPED: setrlimit(RLIMIT_AS) was not permitted");
            return;
        }

        let c_true = code & 1 != 0;
        let r_true = code & 2 != 0;
        let c_moved = code & 4 != 0;
        let r_moved = code & 8 != 0;

        assert_eq!(
            c_true, r_true,
            "C returned {c_true} but Rust returned {r_true} under allocation failure"
        );
        assert_eq!(
            c_moved, r_moved,
            "offset mutation differs under allocation failure (C {c_moved}, Rust {r_moved})"
        );
        assert!(
            !c_true,
            "expected the allocation-failure path (return false); malloc apparently \
             succeeded, so this run did not exercise ERRORS.md row 3"
        );
        assert!(!c_moved, "the malloc-failure path must not advance offset");
        eprintln!("[row 3] allocation failure: C and Rust both returned false, offset untouched");
    }
}

/// A second, allocation-pressure-free check on the same code path: a very large
/// but *satisfiable* `number_string_length`, confirming C and Rust agree when
/// the big `malloc` SUCCEEDS (the other side of the lib.c:64 branch).
#[test]
fn alloc_large_but_successful() {
    let n = 4 << 20; // 4 MiB of digits
    let mut v = vec![b'1'; n];
    v.push(b'z'); // stopper
    // `diff` already asserts full C/Rust equality; these only pin down that we
    // really took the success side of the branch.
    let o = diff("row3/large-ok", Some(&v), v.len(), 0, 0, CJson::garbage());
    assert_eq!(o.ret, 1, "a 4 MiB numeric run must parse");
    assert_eq!(o.buf_offset, n, "the whole digit run is consumed by strtod");
    assert_eq!(o.valueint, i32::MAX, "overflows to +inf -> saturates to INT_MAX");
}

/// ERRORS.md row 10 — `item == NULL` on the SUCCESS path.
///
/// The C never null-checks `item` (lib.c:92 `item->valuedouble = number;`), so a
/// NULL `item` with parsable input faults. This must fault IDENTICALLY in Rust:
/// the translation must not "fix" the bug by adding a null check and returning
/// false. Each implementation is run in its own forked child and we compare the
/// wait status (signal number) the two children die with.
#[test]
fn err_null_item_on_success_path_faults_identically() {
    let p = pair();
    let text: &[u8] = b"123";

    // Returns the raw wait status of a child that calls `f` with item == NULL.
    let run = |which: u8| -> i32 {
        unsafe {
            let pid = fork();
            assert!(pid >= 0, "fork() failed");
            if pid == 0 {
                let mut buf = ParseBuffer {
                    content: text.as_ptr(),
                    length: text.len(),
                    offset: 0,
                    depth: 0,
                };
                let ret = if which == 0 {
                    p.c.parse_number_raw(std::ptr::null_mut(), &mut buf)
                } else {
                    p.rust.parse_number_raw(std::ptr::null_mut(), &mut buf)
                };
                // Reaching here means no fault occurred: report the return value.
                _exit(if ret != 0 { 11 } else { 12 });
            }
            let mut status = 0i32;
            assert_eq!(waitpid(pid, &mut status, 0), pid, "waitpid failed");
            status
        }
    };

    let c_status = run(0);
    let r_status = run(1);

    let sig = |s: i32| s & 0x7f;
    let exited = |s: i32| (s & 0x7f) == 0;
    let code = |s: i32| (s >> 8) & 0xff;

    assert_eq!(
        exited(c_status),
        exited(r_status),
        "one implementation faulted and the other returned normally \
         (C status {c_status:#x}, Rust status {r_status:#x})"
    );

    if exited(c_status) {
        // Both survived: then they must have returned the same sentinel.
        assert_eq!(
            code(c_status),
            code(r_status),
            "NULL item survived in both but with different return values"
        );
        eprintln!(
            "[row 10] NULL item did not fault in either implementation \
             (both returned code {})",
            code(c_status)
        );
    } else {
        assert_eq!(
            sig(c_status),
            sig(r_status),
            "both faulted but with different signals (C {}, Rust {})",
            sig(c_status),
            sig(r_status)
        );
        eprintln!(
            "[row 10] NULL item faults identically in both: signal {}",
            sig(c_status)
        );
    }
}
