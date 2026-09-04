//! Two complementary ways to drive the libraries.
//!
//! 1. **In-process** (`Recorder`): used by the valid-path (Phase B) tests.  A
//!    warning handler records every warning message in order; the error handler
//!    prints the message and aborts, because a `png_error` on a valid input is
//!    itself a bug and must not be silently swallowed.  I/O goes through a
//!    thread-local byte buffer so no `png_get_io_ptr` round-trip is needed and
//!    exactly the same callbacks serve both libraries.
//!
//! 2. **Sub-process** (`run_child`): used by the error-path (Phase C) tests.
//!    `png_error` must not return; the only portable way to observe it from Rust
//!    without `setjmp` is to let the handler print the message and `exit`.  The
//!    parent re-executes the test binary (`--exact harness_child`) once for the
//!    C library and once for the Rust library and compares the two transcripts
//!    line by line, so the error message, the preceding warnings, their order
//!    and the exit code must all agree.
#![allow(dead_code)]

use super::*;
use std::cell::RefCell;
use std::ffi::c_char;
use std::io::Write;

// ---------------------------------------------------------------------------
// thread-local state shared by the callbacks
// ---------------------------------------------------------------------------

thread_local! {
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static IN: RefCell<(Vec<u8>, usize)> = const { RefCell::new((Vec::new(), 0)) };
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// When set, `error_cb` prints the transcript and exits instead of aborting.
    static CHILD_MODE: RefCell<bool> = const { RefCell::new(false) };
    /// Number of rows delivered to the progressive-read row callback, and a
    /// digest of everything the progressive reader handed us.
    static PROG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub fn out_reset() {
    OUT.with(|o| o.borrow_mut().clear());
}
pub fn out_take() -> Vec<u8> {
    OUT.with(|o| std::mem::take(&mut *o.borrow_mut()))
}
pub fn out_len() -> usize {
    OUT.with(|o| o.borrow().len())
}

pub fn in_set(data: &[u8]) {
    IN.with(|i| {
        let mut b = i.borrow_mut();
        b.0 = data.to_vec();
        b.1 = 0;
    });
}
pub fn in_pos() -> usize {
    IN.with(|i| i.borrow().1)
}

pub fn log_reset() {
    LOG.with(|l| l.borrow_mut().clear());
}
pub fn log_push(s: String) {
    LOG.with(|l| l.borrow_mut().push(s));
}
pub fn log_take() -> Vec<String> {
    LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

pub fn prog_reset() {
    PROG.with(|l| l.borrow_mut().clear());
}
pub fn prog_push(s: String) {
    PROG.with(|l| l.borrow_mut().push(s));
}
pub fn prog_take() -> Vec<String> {
    PROG.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

pub fn set_child_mode(on: bool) {
    CHILD_MODE.with(|c| *c.borrow_mut() = on);
}
fn child_mode() -> bool {
    CHILD_MODE.with(|c| *c.borrow())
}

/// Reset everything before a run.
pub fn reset_all() {
    out_reset();
    log_reset();
    prog_reset();
}

// ---------------------------------------------------------------------------
// callbacks (identical code serves both libraries)
// ---------------------------------------------------------------------------

pub unsafe extern "C" fn write_cb(_p: png_structp, data: png_bytep, len: usize) {
    OUT.with(|o| {
        let mut b = o.borrow_mut();
        if len > 0 && !data.is_null() {
            b.extend_from_slice(std::slice::from_raw_parts(data, len));
        }
    });
}

pub unsafe extern "C" fn flush_cb(_p: png_structp) {
    log_push("FLUSH".to_string());
}

/// Reader that satisfies the request from the thread-local input buffer and, if
/// the buffer runs dry, reports a read error the same way `png_default_read_data`
/// would (by calling `png_error`).  Instead of calling back into the library we
/// simply supply zero bytes and let libpng detect the truncation, which is what
/// the deliberately-truncated-stream error rows need.
pub unsafe extern "C" fn read_cb(p: png_structp, data: png_bytep, len: usize) {
    let mut short = false;
    IN.with(|i| {
        let mut b = i.borrow_mut();
        let (buf, pos) = (&b.0, b.1);
        let avail = buf.len().saturating_sub(pos);
        let n = avail.min(len);
        if n > 0 {
            std::ptr::copy_nonoverlapping(buf.as_ptr().add(pos), data, n);
        }
        if n < len {
            // zero-fill the remainder
            std::ptr::write_bytes(data.add(n), 0, len - n);
            short = true;
        }
        b.1 = pos + n;
    });
    if short {
        // mirror png_default_read_data's behaviour on a short read
        let l = libs();
        let f: unsafe extern "C" fn(png_structp, *const c_char) -> ! = if is_c(p) {
            sym(&l.c, "png_error")
        } else {
            sym(&l.rs, "png_error")
        };
        f(p, c"Read Error".as_ptr());
    }
}

/// Which library a `png_structp` belongs to cannot be determined from the
/// pointer, so the tests set this before each run.
thread_local! {
    static CUR_IS_C: RefCell<bool> = const { RefCell::new(true) };
}
pub fn set_cur_is_c(b: bool) {
    CUR_IS_C.with(|c| *c.borrow_mut() = b);
}
fn is_c(_p: png_structp) -> bool {
    CUR_IS_C.with(|c| *c.borrow())
}

pub unsafe extern "C" fn warn_cb(_p: png_structp, msg: png_const_charp) {
    log_push(format!("WARN:{}", cstr_to_string(msg)));
}

pub unsafe extern "C" fn error_cb(_p: png_structp, msg: png_const_charp) -> () {
    let m = cstr_to_string(msg);
    if child_mode() {
        for l in log_take() {
            println!("@@{l}");
        }
        println!("@@ERROR:{m}");
        let _ = std::io::stdout().flush();
        std::process::exit(70);
    } else {
        eprintln!("UNEXPECTED png_error on a valid input: {m}");
        for l in log_take() {
            eprintln!("  prior {l}");
        }
        let _ = std::io::stderr().flush();
        std::process::abort();
    }
}

pub unsafe extern "C" fn read_status_cb(_p: png_structp, row: png_uint_32, pass: c_int) {
    log_push(format!("RSTATUS:{row}:{pass}"));
}
pub unsafe extern "C" fn write_status_cb(_p: png_structp, row: png_uint_32, pass: c_int) {
    log_push(format!("WSTATUS:{row}:{pass}"));
}

// ---------------------------------------------------------------------------
// sub-process driver
// ---------------------------------------------------------------------------

pub const ENV_CASE: &str = "PNG_DIFF_CASE";
pub const ENV_LIB: &str = "PNG_DIFF_LIB";

/// In the child: `Some((case, "c"|"rs"))`.
pub fn child_case() -> Option<(String, String)> {
    match (std::env::var(ENV_CASE), std::env::var(ENV_LIB)) {
        (Ok(c), Ok(l)) => Some((c, l)),
        _ => None,
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Transcript {
    /// `exit(n)` status, `None` when the child died from a signal.
    pub exit: Option<i32>,
    /// The terminating signal, when any.  `PNG_ABORT()` (used by `png_longjmp`
    /// when no `jmp_buf` was installed) legitimately raises SIGABRT, and that is
    /// itself a rejection whose equality must be checked.
    pub signal: Option<i32>,
    pub lines: Vec<String>,
}

/// Re-execute this very test binary to run `case` against library `which`.
pub fn run_child(case: &str, which: &str) -> Transcript {
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["--exact", "harness_child", "--nocapture", "--test-threads=1"])
        .env(ENV_CASE, case)
        .env(ENV_LIB, which)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn child");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    // libtest prints `test harness_child ... ` WITHOUT a trailing newline, so the
    // first transcript line is not at the start of its line.  Match the `@@`
    // marker anywhere.
    let lines: Vec<String> = stdout
        .lines()
        .filter_map(|l| l.find("@@").map(|i| l[i + 2..].to_string()))
        .filter(|l| !l.is_empty())
        .collect();
    use std::os::unix::process::ExitStatusExt;
    Transcript {
        exit: out.status.code(),
        signal: out.status.signal(),
        lines,
    }
}

/// Run `case` against both libraries and assert the transcripts are identical.
#[track_caller]
pub fn diff_case(case: &str) {
    let c = run_child(case, "c");
    let r = run_child(case, "rs");
    if c != r {
        panic!(
            "ERROR-PATH MISMATCH for case {case:?}\n  C   : exit={:?} signal={:?} lines={:#?}\n  RUST: exit={:?} signal={:?} lines={:#?}",
            c.exit, c.signal, c.lines, r.exit, r.signal, r.lines
        );
    }
    // A case that produces NO transcript at all AND no clean exit observed
    // nothing; that would be a vacuous "pass", so reject it.
    assert!(
        !(c.lines.is_empty() && c.exit.is_none()),
        "case {case:?} died from signal {:?} in BOTH libraries without emitting \
         anything - nothing was actually compared",
        c.signal
    );
}

/// Emit a transcript line from inside a child.
pub fn emit(s: impl AsRef<str>) {
    println!("@@{}", s.as_ref());
}

/// Flush the recorded warning log to the transcript (children only).
pub fn emit_log() {
    for l in log_take() {
        emit(l);
    }
}

pub fn child_finish() -> ! {
    emit_log();
    emit("OK");
    let _ = std::io::stdout().flush();
    std::process::exit(0);
}
