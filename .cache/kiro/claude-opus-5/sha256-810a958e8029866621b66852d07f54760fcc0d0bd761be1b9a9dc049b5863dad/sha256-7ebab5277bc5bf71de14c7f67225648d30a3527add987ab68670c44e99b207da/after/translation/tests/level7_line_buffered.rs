//! Level 7: `LOG_FILE` pointing at a terminal.
//!
//! `fopen()` on a character device that is a tty makes glibc pick a *line*
//! buffered stream sized from `st_blksize` (1024 for a pty slave) instead of a
//! fully buffered 4096-byte one, so each log entry reaches the device as soon
//! as its newline is written.  The translation has to make the same choice.

mod harness;

use harness::{cstr, show, Api};
use std::ffi::{c_char, c_int};

extern "C" {
    fn posix_openpt(flags: c_int) -> c_int;
    fn grantpt(fd: c_int) -> c_int;
    fn unlockpt(fd: c_int) -> c_int;
    fn ptsname(fd: c_int) -> *mut c_char;
    fn read(fd: c_int, buf: *mut u8, count: usize) -> isize;
    fn close(fd: c_int) -> c_int;
    fn fcntl(fd: c_int, cmd: c_int, arg: c_int) -> c_int;
}

const O_RDWR: c_int = 0o2;
const O_NOCTTY: c_int = 0o400;
const F_SETFL: c_int = 4;
const O_NONBLOCK: c_int = 0o4000;

struct Pty {
    master: c_int,
    slave_path: String,
}

impl Pty {
    fn new() -> Pty {
        unsafe {
            let master = posix_openpt(O_RDWR | O_NOCTTY);
            assert!(master >= 0, "posix_openpt failed");
            assert_eq!(grantpt(master), 0, "grantpt failed");
            assert_eq!(unlockpt(master), 0, "unlockpt failed");
            let name = ptsname(master);
            assert!(!name.is_null(), "ptsname failed");
            let slave_path = std::ffi::CStr::from_ptr(name)
                .to_str()
                .unwrap()
                .to_string();
            // Non-blocking so `drain` never hangs when there is nothing to read.
            fcntl(master, F_SETFL, O_NONBLOCK);
            Pty { master, slave_path }
        }
    }

    /// Everything the device has received so far.
    fn drain(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = unsafe { read(self.master, buf.as_mut_ptr(), buf.len()) };
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        out
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        unsafe {
            close(self.master);
        }
    }
}

/// Runs `scenario` against both libraries with `LOG_FILE` pointing at a fresh
/// pty slave, and compares what the terminal actually received at each step.
fn compare_on_pty<F>(case: &str, scenario: F)
where
    F: Fn(&Api, &Pty, &mut Vec<String>),
{
    let _guard = harness::lock();
    harness::ensure_built();
    let libs = [harness::load_single("c"), harness::load_single("rust")];
    let mut results = Vec::new();
    for api in libs {
        let pty = Pty::new();
        harness::env_set("LOG_FILE", Some(&pty.slave_path));
        harness::env_set("MAX_TASKS", None);
        let mut transcript = Vec::new();
        let ((), _cap) = harness::capture(|| scenario(api, &pty, &mut transcript));
        transcript.push(format!("final drain: {}", show(&pty.drain())));
        results.push(transcript);
    }
    assert_eq!(
        results[0], results[1],
        "case `{case}` diverged on a tty log target:\n  C   : {:#?}\n  Rust: {:#?}",
        results[0], results[1]
    );
}

/// Each entry must appear on the terminal as soon as it is logged.
#[test]
fn each_entry_reaches_the_terminal_immediately() {
    compare_on_pty("line buffered entries", |api, pty, t| unsafe {
        t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
        t.push(format!("after init: {}", show(&pty.drain())));
        for i in 0..5 {
            let m = cstr(format!("entry number {i}").as_bytes());
            (api.log_info)(m.as_ptr());
            t.push(format!("after log {i}: {}", show(&pty.drain())));
        }
        let w = cstr(b"careful");
        (api.log_warning)(w.as_ptr());
        t.push(format!("after warning: {}", show(&pty.drain())));
        (api.finalize_logger)();
        t.push(format!("after finalize: {}", show(&pty.drain())));
    });
}

/// A message with embedded newlines: everything up to and including the last
/// newline is flushed.
#[test]
fn embedded_newlines_on_a_terminal() {
    compare_on_pty("embedded newlines", |api, pty, t| unsafe {
        t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
        t.push(format!("after init: {}", show(&pty.drain())));
        for msg in [
            &b"one\ntwo"[..],
            &b"three\n"[..],
            &b"\n\n\n"[..],
            &b"plain"[..],
        ] {
            let m = cstr(msg);
            (api.log_info)(m.as_ptr());
            t.push(format!("after {:?}: {}", show(msg), show(&pty.drain())));
        }
        (api.finalize_logger)();
        t.push(format!("after finalize: {}", show(&pty.drain())));
    });
}

/// A single entry larger than the device's 1024-byte stdio buffer.
#[test]
fn oversized_entry_on_a_terminal() {
    for len in [900usize, 1016, 1017, 1024, 2048, 3000] {
        compare_on_pty(&format!("oversized entry {len}"), move |api, pty, t| unsafe {
            t.push(format!("initialize_logger -> {}", (api.initialize_logger)()));
            let mut got = pty.drain();
            let m = cstr(&vec![b'T'; len]);
            (api.log_info)(m.as_ptr());
            // The terminal only holds ~4 KiB, so read as we go.
            got.extend_from_slice(&pty.drain());
            t.push(format!("received {} bytes", got.len()));
            (api.finalize_logger)();
            got.extend_from_slice(&pty.drain());
            t.push(format!("total {} bytes", got.len()));
            t.push(format!("digest {:?}", digest(&got)));
        });
    }
}

/// `driver()` end to end with the log going to a terminal.
#[test]
fn driver_logging_to_a_terminal() {
    compare_on_pty("driver on a tty", |api, pty, t| unsafe {
        harness::env_set("MAX_TASKS", Some("3"));
        let s = cstr(b"alpha\nbeta\ngamma\ndelta\n");
        t.push(format!("driver -> {}", (api.driver)(s.as_ptr())));
        t.push(format!("terminal: {}", show(&pty.drain())));
    });
}

fn digest(bytes: &[u8]) -> String {
    // Cheap content fingerprint: length plus a rolling sum, enough to notice a
    // difference without dumping kilobytes into the failure message.
    let mut h: u64 = 1469598103934665603;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    format!("{}:{:016x}", bytes.len(), h)
}

extern "C" {
    fn open(path: *const c_char, flags: c_int, mode: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    static stdout: *mut std::ffi::c_void;
}

const O_WRONLY: c_int = 0o1;
const O_APPEND: c_int = 0o2000;

/// `print_tasks` writes through the C runtime's `stdout` while the logger has
/// its own `FILE *`.  When both end up on the same terminal, the order the two
/// buffers reach it is observable — and must be the same for both libraries.
#[test]
fn stdout_and_log_share_a_terminal() {
    let _guard = harness::lock();
    harness::ensure_built();
    let libs = [harness::load_single("c"), harness::load_single("rust")];
    let mut results: Vec<Vec<u8>> = Vec::new();

    for api in libs {
        let pty = Pty::new();
        harness::env_set("LOG_FILE", Some(&pty.slave_path));
        harness::env_set("MAX_TASKS", Some("3"));

        let slave_cstr = std::ffi::CString::new(pty.slave_path.clone()).unwrap();
        let mut received = Vec::new();
        unsafe {
            let slave = open(slave_cstr.as_ptr(), O_WRONLY | O_APPEND | O_NOCTTY, 0);
            assert!(slave >= 0, "opening the pty slave failed");
            fflush(stdout);
            let saved = dup(1);
            dup2(slave, 1);

            let s = cstr(b"alpha\nbeta\ngamma\ndelta\n");
            let rc = (api.driver)(s.as_ptr());

            fflush(stdout);
            dup2(saved, 1);
            close(saved);
            close(slave);
            received.extend_from_slice(&pty.drain());
            received.extend_from_slice(format!("|rc={rc}").as_bytes());
        }
        results.push(received);
    }

    assert_eq!(
        results[0],
        results[1],
        "stdout/log interleaving on a shared terminal diverged:\n  C   : {}\n  Rust: {}",
        show(&results[0]),
        show(&results[1])
    );
}
