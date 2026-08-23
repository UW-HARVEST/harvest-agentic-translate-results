//! Differential comparison in a forked child.
//!
//! Some inputs are fatal to the C library — it dereferences a NULL, walks off the
//! end of a buffer, or reaches `PNG_ABORT()`.  Those are still *observations*
//! that the Rust translation has to reproduce, so each side of the comparison is
//! run in its own `fork()`ed child: the parent then compares the exit status, the
//! terminating signal, and everything the child wrote (its event trace plus
//! whatever the library printed on stdout/stderr).
#![allow(dead_code)]

use super::*;
use core::ffi::c_int;
use std::io::Write;
use std::os::unix::io::AsRawFd;

extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
    fn dup2(old: i32, new: i32) -> i32;
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ChildResult {
    /// `Some(exit code)`, or `None` if the child died from a signal.
    pub exit: Option<i32>,
    pub signal: i32,
    pub text: String,
}

pub fn scratch_dir() -> std::path::PathBuf {
    let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/forked");
    let _ = std::fs::create_dir_all(&d);
    d
}

static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Run `body` in a forked child and capture everything it produced.
pub fn run_in_child(body: &mut dyn FnMut() -> String) -> ChildResult {
    // Make sure every process-global lazy singleton is fully initialised *before*
    // the fork, so the child never has to take a lock another thread may have been
    // holding at the instant of the fork.
    let _ = libs();
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = scratch_dir().join(format!("c{}-{}.txt", std::process::id(), n));
    let _ = std::fs::remove_file(&path);
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // ---- child ----
        IN_FORKED_CHILD.store(true, std::sync::atomic::Ordering::Relaxed);
        let f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .expect("open capture file");
        let fd = f.as_raw_fd();
        unsafe {
            dup2(fd, 1);
            dup2(fd, 2);
        }
        let mut state = Box::new(Tls::default());
        set_tls(&mut *state as *mut Tls);
        let r = body();
        let mut out = String::new();
        for e in &state.trace {
            out.push_str(e);
            out.push('\n');
        }
        out.push_str("=> ");
        out.push_str(&r);
        out.push('\n');
        let mut g = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("reopen capture file");
        let _ = g.write_all(out.as_bytes());
        let _ = g.flush();
        drop(g);
        unsafe { _exit(0) }
    }
    // ---- parent ----
    let mut status = 0i32;
    let r = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(r, pid, "waitpid");
    let text = std::fs::read(&path)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    if status & 0x7f == 0 {
        ChildResult {
            exit: Some((status >> 8) & 0xff),
            signal: 0,
            text,
        }
    } else {
        ChildResult {
            exit: None,
            signal: status & 0x7f,
            text,
        }
    }
}

pub static FORKED_CASES: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Run `f` for both libraries, each in its own child, and require the same
/// result — including dying from the same signal.
#[track_caller]
pub fn assert_same_forked<F>(case: &str, f: F)
where
    F: Fn(&Api) -> String,
{
    let l = libs();
    let mut res = Vec::new();
    for api in [&l.c, &l.rust] {
        res.push(run_in_child(&mut || {
            set_cur_api(api as *const Api);
            f(api)
        }));
    }
    FORKED_CASES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    assert!(
        res[0] == res[1],
        "forked case `{}` diverged\n  C   : exit={:?} signal={} text={:?}\n  Rust: exit={:?} signal={} text={:?}",
        case,
        res[0].exit,
        res[0].signal,
        res[0].text,
        res[1].exit,
        res[1].signal,
        res[1].text
    );
}

/// The common shape: create a struct, run `body` under the error trap, report.
pub fn guarded_in_child(
    api: &Api,
    write: bool,
    body: &mut dyn FnMut(&Api, *mut PngStruct, *mut PngInfo),
) -> String {
    unsafe {
        let (png, info) = if write { new_write(api) } else { new_read(api) };
        let g = guarded(api, png, &mut || body(api, png, info));
        let extra = format!("{:?} out={} bytes", g, tls().output.len());
        // NB: no destroy -- the child is about to _exit and the struct may be in
        // an undefined state after a fatal error, exactly as in a real crash.
        let _ = (png, info);
        extra
    }
}

pub const _UNUSED_FORKED: c_int = 0;
