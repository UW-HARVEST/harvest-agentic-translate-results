//! Phase C (rows B3-B5) — NULL-pointer boundaries.
//!
//! `strlen(NULL)` / `strtol(NULL, ...)` fault in C, so the differential check
//! is "both libraries die from the same signal". Each call therefore runs in a
//! forked child. Everything lives in a single `#[test]` so no other test thread
//! can be running while this process forks.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

const PROG: &[u8] = b"driver";
const SIGSEGV: i32 = 11;

/// How the library terminates a process when called with `(argc, argv)`.
/// The child's stdout is captured so the surviving cases stay quiet.
fn death_of(lib: &Lib, argc: c_int, argv: *mut *mut c_char) -> Result<i32, i32> {
    let (how, _out) = capture_stdout(|| {
        fork_and_run(|| {
            let _ = unsafe { lib.call(argc, argv) };
        })
    });
    how
}

#[test]
fn boundary_b3_b4_b5_null_pointers() {
    let (c_lib, rust_lib) = libs();

    // B3: argv[1] == NULL, for every argc that reaches strlen(argv[1]).
    for argc in [2, 3, 4] {
        let mut argv = Argv::new(&[PROG, b"abc", b"1", b"2"], Layout::Contiguous);
        argv.set(1, std::ptr::null_mut());
        let p = argv.as_ptr();
        let c = death_of(c_lib, argc, p);
        let r = death_of(rust_lib, argc, p);
        assert_eq!(c, Err(SIGSEGV), "C must segfault on argv[1] == NULL");
        assert_eq!(r, c, "argc={argc}: Rust must die exactly like C (B3)");
    }

    // B4: argv[2] == NULL with argc >= 3.
    for argc in [3, 4] {
        let mut argv = Argv::new(&[PROG, b"abc", b"1", b"2"], Layout::Contiguous);
        argv.set(2, std::ptr::null_mut());
        let p = argv.as_ptr();
        let c = death_of(c_lib, argc, p);
        let r = death_of(rust_lib, argc, p);
        assert_eq!(c, Err(SIGSEGV), "C must segfault on argv[2] == NULL");
        assert_eq!(r, c, "argc={argc}: Rust must die exactly like C (B4)");
    }

    // B5: argv[3] == NULL with argc == 4.
    let mut argv = Argv::new(&[PROG, b"abc", b"1", b"2"], Layout::Contiguous);
    argv.set(3, std::ptr::null_mut());
    let p = argv.as_ptr();
    let c = death_of(c_lib, 4, p);
    let r = death_of(rust_lib, 4, p);
    assert_eq!(c, Err(SIGSEGV), "C must segfault on argv[3] == NULL");
    assert_eq!(r, c, "Rust must die exactly like C (B5)");

    // A NULL argv[1] that is never read (argc == 1, argc == 5) must NOT crash.
    for argc in [1, 5] {
        let mut argv = Argv::new(&[PROG, b"abc", b"1", b"2"], Layout::Contiguous);
        argv.set(1, std::ptr::null_mut());
        let p = argv.as_ptr();
        let c = death_of(c_lib, argc, p);
        let r = death_of(rust_lib, argc, p);
        assert_eq!(c, Ok(0), "argc={argc} only prints the usage message");
        assert_eq!(r, c, "argc={argc}: Rust must survive exactly like C");
    }

    // The forking machinery itself must be trustworthy: a clean call exits 0.
    let mut argv = Argv::new(&[PROG, b"abc"], Layout::Contiguous);
    let p = argv.as_ptr();
    assert_eq!(death_of(c_lib, 2, p), Ok(0));
    assert_eq!(death_of(rust_lib, 2, p), Ok(0));
}

extern "C" {
    fn mmap(
        addr: *mut std::ffi::c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        off: i64,
    ) -> *mut std::ffi::c_void;
    fn munmap(addr: *mut std::ffi::c_void, len: usize) -> c_int;
}

const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const MAP_PRIVATE: c_int = 2;
const MAP_ANONYMOUS: c_int = 0x20;

/// Two consecutive pages, of which the second one is unmapped: anything that
/// reads past the end of the first page faults.
struct GuardedPage {
    base: *mut u8,
    page: usize,
}

impl GuardedPage {
    fn new() -> GuardedPage {
        let page = 4096usize;
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
        assert!(base as isize > 0, "mmap failed");
        // drop the second page so it becomes a guard
        assert_eq!(
            unsafe { munmap((base as *mut u8).add(page) as *mut _, page) },
            0
        );
        GuardedPage {
            base: base as *mut u8,
            page,
        }
    }

    /// Write `bytes` so that its last byte is the last byte of the mapped page.
    fn place_at_end(&self, bytes: &[u8]) -> *mut c_char {
        assert!(bytes.len() <= self.page);
        let start = unsafe { self.base.add(self.page - bytes.len()) };
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), start, bytes.len()) };
        start as *mut c_char
    }
}

impl Drop for GuardedPage {
    fn drop(&mut self) {
        unsafe { munmap(self.base as *mut _, self.page) };
    }
}

/// B12 — a string whose NUL terminator is the very last readable byte before an
/// unmapped page. Neither implementation may read past it.
#[test]
fn boundary_b12_string_at_page_boundary() {
    let guard = GuardedPage::new();
    let mut with_nul = b"abcdef".to_vec();
    with_nul.push(0);
    let s = guard.place_at_end(&with_nul);

    for (argc, extra) in [(2, vec![]), (3, vec![b"2".to_vec()]), (4, vec![b"2".to_vec(), b"5".to_vec()])] {
        let mut owned: Vec<Vec<u8>> = extra
            .into_iter()
            .map(|mut v: Vec<u8>| {
                v.push(0);
                v
            })
            .collect();
        let mut ptrs: Vec<*mut c_char> = vec![PROG.as_ptr() as *mut c_char, s];
        for o in owned.iter_mut() {
            ptrs.push(o.as_mut_ptr() as *mut c_char);
        }
        ptrs.push(std::ptr::null_mut());
        // PROG is a &[u8] literal without a NUL; give argv[0] a real C string.
        let mut prog = b"driver\0".to_vec();
        ptrs[0] = prog.as_mut_ptr() as *mut c_char;

        let mut argv = Argv::from_raw_ptrs(ptrs, argc);
        let out = assert_same_argv(&mut argv, argc, "string ending at a page boundary");
        assert_eq!(out.status, 0);
        let expected: &[u8] = match argc {
            2 => b"abcdef\n",
            3 => b"cdef\n",
            _ => b"cde\n",
        };
        assert_eq!(out.stdout, expected);
    }
}

/// B13 — the same page, but the string is *not* terminated: both
/// implementations must run into the guard page and die identically.
#[test]
fn boundary_b13_unterminated_string_faults_identically() {
    let (c_lib, rust_lib) = libs();
    let guard = GuardedPage::new();
    let filler = vec![b'z'; 64];
    let s = guard.place_at_end(&filler);
    let mut prog = b"driver\0".to_vec();
    let ptrs: Vec<*mut c_char> = vec![
        prog.as_mut_ptr() as *mut c_char,
        s,
        std::ptr::null_mut(),
    ];
    let mut argv = Argv::from_raw_ptrs(ptrs, 2);
    let p = argv.as_ptr();
    let c = death_of(c_lib, 2, p);
    let r = death_of(rust_lib, 2, p);
    assert_eq!(c, Err(SIGSEGV), "the C strlen must run off the page");
    assert_eq!(r, c, "Rust must fault exactly like C");
}

/// B11 — `argv` itself is NULL.
#[test]
fn boundary_b11_null_argv() {
    let (c_lib, rust_lib) = libs();
    // argc == 1 and argc > 4 return before argv is ever touched.
    for argc in [1, 5, 100] {
        let c = death_of(c_lib, argc, std::ptr::null_mut());
        let r = death_of(rust_lib, argc, std::ptr::null_mut());
        assert_eq!(c, Ok(0), "argc={argc} must not touch argv");
        assert_eq!(r, c, "argc={argc}: Rust must behave like C with argv == NULL");
    }
    // every other argc dereferences argv[1] and faults
    for argc in [-1, 0, 2, 3, 4] {
        let c = death_of(c_lib, argc, std::ptr::null_mut());
        let r = death_of(rust_lib, argc, std::ptr::null_mut());
        assert_eq!(c, Err(SIGSEGV), "argc={argc} must fault in C");
        assert_eq!(r, c, "argc={argc}: Rust must fault exactly like C");
    }
}
