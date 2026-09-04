//! `ERRORS.md` rows E24a and E25 — behaviour that cannot be observed safely
//! in-process, verified by running each library in its own forked child.
//!
//! * **E24a**: pointers the C dereferences without a guard. The C has no NULL
//!   check, so the correct Rust behaviour is to fault identically — not to
//!   return an error. Asserted by comparing the children's wait statuses.
//! * **E25**: `matrix_to_string`'s buffer formula budgets `11*width` bytes per
//!   row but a row of 11-character values needs `12*width`, so the C overflows
//!   its heap buffer. Each library runs in a separate child so the corruption
//!   cannot cross-contaminate, and the produced bytes are compared.

mod common;

use common::*;
use std::ffi::c_int;
use std::os::fd::AsRawFd;
use std::ptr;
use std::sync::{Mutex, MutexGuard, OnceLock};

unsafe extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn write(fd: c_int, buf: *const u8, count: usize) -> isize;
}

/// fork() in a threaded process is only safe if nothing else is running.
fn serial() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Exited(c_int),
    Signalled(c_int),
}

fn decode(status: c_int) -> Outcome {
    if status & 0x7f == 0x7f {
        // stopped — not expected here
        Outcome::Signalled(-1)
    } else if status & 0x7f == 0 {
        Outcome::Exited((status >> 8) & 0xff)
    } else {
        Outcome::Signalled(status & 0x7f)
    }
}

/// Forks, runs `child` (which must not return normally without calling
/// `_exit`), and returns the child's outcome plus whatever it wrote to `out_fd`.
fn run_in_child(body: impl FnOnce(c_int)) -> (Outcome, Vec<u8>) {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("driver_child_{}_{n}", std::process::id()));
    let file = std::fs::File::create(&path).expect("create child output file");
    let fd = file.as_raw_fd();
    // Everything the child needs is already allocated; after fork it only calls
    // the library under test, write(2) and _exit(2).
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        body(fd);
        unsafe { _exit(0) };
    }
    let mut status: c_int = 0;
    let r = unsafe { waitpid(pid, &mut status as *mut c_int, 0) };
    assert_eq!(r, pid, "waitpid failed");
    drop(file);
    let bytes = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    (decode(status), bytes)
}

// ---------------------------------------------------------------------------
// E24a — NULL-pointer dereference parity
// ---------------------------------------------------------------------------

#[test]
fn err_e24a_null_deref_parity() {
    let _g = serial();
    let [c, r] = both();
    let ok = cstring("1 2\n3 4\n");

    // Each case is a closure over an index selecting the call to make.
    // 0: initialize_matrix_from_string(NULL, 1, 1)
    // 1: initialize_matrix_from_string(NULL, 0, 0)
    // 2: multiply_matrices(NULL, valid)
    // 3: multiply_matrices(valid, NULL)
    // 4: driver(..., NULL, ...) for matrix_a
    // 5: driver(..., NULL) for matrix_b
    for case in 0..6 {
        let mut outcomes = Vec::new();
        for api in [c, r] {
            // Build the "valid" operand *before* forking so the child performs
            // no allocation of its own.
            let valid = if case == 2 || case == 3 {
                unsafe { (api.initialize_matrix_from_string)(ok.as_ptr(), 2, 2) }
            } else {
                ptr::null_mut()
            };
            if case == 2 || case == 3 {
                assert!(!valid.is_null(), "{}: setup failed", api.name);
            }
            let (outcome, bytes) = run_in_child(|fd| unsafe {
                let marker: &[u8] = b"returned";
                match case {
                    0 => {
                        let m = (api.initialize_matrix_from_string)(ptr::null(), 1, 1);
                        let tag = if m.is_null() { b"null\0" } else { b"some\0" };
                        write(fd, marker.as_ptr(), marker.len());
                        write(fd, tag.as_ptr(), 4);
                    }
                    1 => {
                        let m = (api.initialize_matrix_from_string)(ptr::null(), 0, 0);
                        let tag = if m.is_null() { b"null\0" } else { b"some\0" };
                        write(fd, marker.as_ptr(), marker.len());
                        write(fd, tag.as_ptr(), 4);
                    }
                    2 => {
                        let m = (api.multiply_matrices)(ptr::null_mut(), valid);
                        let tag = if m.is_null() { b"null\0" } else { b"some\0" };
                        write(fd, marker.as_ptr(), marker.len());
                        write(fd, tag.as_ptr(), 4);
                    }
                    3 => {
                        let m = (api.multiply_matrices)(valid, ptr::null_mut());
                        let tag = if m.is_null() { b"null\0" } else { b"some\0" };
                        write(fd, marker.as_ptr(), marker.len());
                        write(fd, tag.as_ptr(), 4);
                    }
                    4 => {
                        let rc = (api.driver)(2, 2, ptr::null(), 2, 2, ok.as_ptr());
                        let b = [b'r', b'c', b'=', (b'0' + (rc as u8 & 0x0f))];
                        write(fd, b.as_ptr(), b.len());
                    }
                    _ => {
                        let rc = (api.driver)(2, 2, ok.as_ptr(), 2, 2, ptr::null());
                        let b = [b'r', b'c', b'=', (b'0' + (rc as u8 & 0x0f))];
                        write(fd, b.as_ptr(), b.len());
                    }
                }
                _exit(0);
            });
            if !valid.is_null() {
                unsafe { (api.free_matrix)(valid) };
            }
            outcomes.push((outcome, bytes));
        }
        assert_eq!(
            outcomes[0], outcomes[1],
            "case {case}: C and Rust diverged on a NULL dereference"
        );
        // Every one of these cases dereferences NULL in the C, so the expected
        // outcome is a fatal signal, not a graceful error return.
        assert!(
            matches!(outcomes[0].0, Outcome::Signalled(_)),
            "case {case}: expected a fatal signal, got {:?} ({:?})",
            outcomes[0].0,
            String::from_utf8_lossy(&outcomes[0].1)
        );
    }
}

// ---------------------------------------------------------------------------
// E12b / E23b — unchecked-allocation crashes, verified for parity in isolation
//
// The C never checks `allocate_matrix`/`malloc` results at these two call
// sites, so when the allocation *fails* but the following loop still runs, the
// C dereferences a NULL pointer. Both libraries must fault the same way.
// ---------------------------------------------------------------------------

#[test]
fn err_e12b_unchecked_allocation_crash_parity() {
    let _g = serial();

    // matrix_to_string: buffer_size wraps twice and lands back on a positive
    // value (width=200000000, height=2 -> +105032704), so malloc succeeds and
    // the loop dereferences the NULL `matrix` field.
    for &(width, height) in &[(200_000_000, 2), (195_225_785, 1), (100_000_000, 4)] {
        let mut outcomes = Vec::new();
        for api in both() {
            let m = Box::new(MatrixT {
                matrix: ptr::null_mut(),
                width,
                height,
            });
            let p = Box::into_raw(m);
            let (outcome, bytes) = run_in_child(|fd| unsafe {
                let s = (api.matrix_to_string)(p);
                if s.is_null() {
                    let tag = b"NULL";
                    write(fd, tag.as_ptr(), tag.len());
                } else {
                    let tag = b"OK";
                    write(fd, tag.as_ptr(), tag.len());
                }
                _exit(0);
            });
            drop(unsafe { Box::from_raw(p) });
            outcomes.push((outcome, bytes));
        }
        assert_eq!(
            outcomes[0], outcomes[1],
            "matrix_to_string(width={width}, height={height}) diverged in isolation"
        );
        assert!(
            matches!(outcomes[0].0, Outcome::Signalled(_)),
            "expected a fatal signal for width={width} height={height}, got {:?}",
            outcomes[0].0
        );
    }
}

#[test]
fn err_e23b_init_unchecked_allocation_crash_parity() {
    let _g = serial();
    // allocate_matrix's row allocation fails (width * 4 is ~8 GiB), the C does
    // not check it, and the column loop then writes through the NULL matrix.
    let cases: &[(&str, c_int, c_int)] = &[
        ("1 2 3\n4 5 6\n", i32::MAX, 1),
        ("1 2 3\n4 5 6\n", i32::MAX - 1, 2),
        ("1\n", i32::MAX, 1),
    ];
    for &(input, w, h) in cases {
        let input_c = cstring(input);
        let mut outcomes = Vec::new();
        for api in both() {
            let (outcome, bytes) = run_in_child(|fd| unsafe {
                let m = (api.initialize_matrix_from_string)(input_c.as_ptr(), w, h);
                let tag: &[u8] = if m.is_null() { b"NULL" } else { b"OK" };
                write(fd, tag.as_ptr(), tag.len());
                _exit(0);
            });
            outcomes.push((outcome, bytes));
        }
        assert_eq!(
            outcomes[0], outcomes[1],
            "initialize_matrix_from_string({input:?}, {w}, {h}) diverged in isolation"
        );
        assert!(
            matches!(outcomes[0].0, Outcome::Signalled(_)),
            "expected a fatal signal for ({input:?}, {w}, {h}), got {:?} ({:?})",
            outcomes[0].0,
            String::from_utf8_lossy(&outcomes[0].1)
        );
    }
}


// ---------------------------------------------------------------------------
// E25 — matrix_to_string heap-buffer overflow parity
// ---------------------------------------------------------------------------

#[test]
fn err_e25_buffer_overflow_parity() {
    let _g = serial();
    // (height, width, value) triples whose rendering exceeds the C's buffer:
    // needed = height*(11*width + width) + 1, budget = height*11*width+height+1
    let cases: &[(usize, usize, c_int)] = &[
        (1, 2, i32::MIN),
        (1, 3, i32::MIN),
        (2, 2, i32::MIN),
        (3, 4, -1_000_000_000),
        (1, 8, i32::MIN + 1),
    ];
    for &(h, w, v) in cases {
        let rows: Vec<Vec<c_int>> = (0..h).map(|_| vec![v; w]).collect();
        let mut results = Vec::new();
        for api in both() {
            let mat = unsafe { build_matrix(api, &rows, w as c_int) };
            let (outcome, bytes) = run_in_child(|fd| unsafe {
                let s = (api.matrix_to_string)(mat);
                if s.is_null() {
                    let tag = b"NULL";
                    write(fd, tag.as_ptr(), tag.len());
                } else {
                    let len = strlen(s);
                    write(fd, s as *const u8, len);
                }
                // Deliberately no free(): the C has already written past the end
                // of the chunk, so tearing the heap down would only add noise.
                _exit(0);
            });
            unsafe { (api.free_matrix)(mat) };
            results.push((outcome, bytes));
        }
        assert_eq!(
            (
                &results[0].0,
                String::from_utf8_lossy(&results[0].1).into_owned()
            ),
            (
                &results[1].0,
                String::from_utf8_lossy(&results[1].1).into_owned()
            ),
            "matrix_to_string overflow case {h}x{w} value {v} diverged"
        );
        // Sanity: the rendering really is longer than the C's own budget.
        let budget = (h as i64) * (11 * w as i64) + h as i64 + 1;
        let needed = (h as i64) * (12 * w as i64) + 1;
        assert!(
            needed > budget,
            "case {h}x{w} does not actually overflow (needed {needed}, budget {budget})"
        );
        assert_eq!(
            results[0].1.len() as i64,
            needed - 1,
            "unexpected rendering length for {h}x{w}"
        );
    }
}
