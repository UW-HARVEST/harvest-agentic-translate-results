//! Shared harness: loads the C and Rust shared objects via `libloading` and
//! captures everything each one writes to `stdout`, plus how it terminated, so
//! the two can be compared exactly.
//!
//! Each call runs in a `fork()`ed child whose fd 1 is a temporary file. That
//! isolates the capture from libtest's own progress output on stdout and keeps
//! a crash in either library (the C `bad()` can segfault for out-of-bounds
//! indices) from taking down the test process.

#![allow(dead_code)]

use std::ffi::{c_char, c_int};
use std::io::Read;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use libloading::{Library, Symbol};

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("workspace root")
        .join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    // The cdylib produced by this crate lives in target/<profile>/, one level
    // above the test binary in target/<profile>/deps/.
    let exe = std::env::current_exe().expect("test exe path");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    let candidate = dir.join("libdriver.so");
    assert!(
        candidate.exists(),
        "Rust cdylib not found at {}. Run `cargo build` before `cargo test`.",
        candidate.display()
    );
    candidate
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library missing at {}; build it with cmake first.",
            c_path.display()
        );
        // RTLD_LOCAL (libloading's default) keeps the two definitions of
        // `printLine` / `printIntLine` from shadowing each other.
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
        Libs { c, rust }
    })
}

/// Signatures for the non-static functions in `src/driver.c`.
pub type FnVoidCharPtr = unsafe extern "C" fn(*const c_char);
pub type FnVoidInt = unsafe extern "C" fn(c_int);
pub type FnVoidIntInt = unsafe extern "C" fn(c_int, c_int);

pub unsafe fn sym<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    unsafe {
        lib.get(name.as_bytes())
            .unwrap_or_else(|e| panic!("symbol `{name}` not found: {e}"))
    }
}

/// What a captured call produced.
#[derive(PartialEq, Eq)]
pub struct Outcome {
    /// Bytes written to fd 1.
    pub stdout: Vec<u8>,
    /// Raw `waitpid` status of the child (0 = clean exit).
    pub status: c_int,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Outcome {{ status: {}, stdout: \"{}\" }}",
            describe_status(self.status),
            show(&self.stdout)
        )
    }
}

pub fn describe_status(status: c_int) -> String {
    if libc::WIFEXITED(status) {
        format!("exit({})", libc::WEXITSTATUS(status))
    } else if libc::WIFSIGNALED(status) {
        format!("signal({})", libc::WTERMSIG(status))
    } else {
        format!("raw({status})")
    }
}

/// Run `f` in a forked child with fd 1 redirected to a temporary file; return
/// the captured bytes and the child's wait status.
pub fn capture<F: FnOnce()>(f: F) -> Outcome {
    static SEQ: AtomicU64 = AtomicU64::new(0);

    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "driver_capture_{}_{}.out",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));

    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&tmp)
        .expect("open capture file");
    let target = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    // Flush before forking so buffered parent output is not duplicated.
    unsafe { libc::fflush(std::ptr::null_mut()) };
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // Child: redirect stdout, run the call, flush stdio, exit immediately
        // without unwinding or running libtest teardown.
        unsafe {
            if libc::dup2(target, 1) < 0 {
                libc::_exit(101);
            }
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
            libc::fflush(std::ptr::null_mut());
            libc::_exit(if r.is_ok() { 0 } else { 102 });
        }
    }

    let mut status: c_int = 0;
    let waited = unsafe { libc::waitpid(pid, &mut status, 0) };
    assert_eq!(waited, pid, "waitpid failed");

    let mut file = file;
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::Start(0)).expect("rewind capture");
    let mut stdout = Vec::new();
    file.read_to_end(&mut stdout).expect("read capture");
    drop(file);
    let _ = std::fs::remove_file(&tmp);

    Outcome { stdout, status }
}

/// Convenience wrapper for callers that only care about the bytes.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let o = capture(f);
    assert_eq!(
        o.status,
        0,
        "captured call terminated abnormally: {}",
        describe_status(o.status)
    );
    o.stdout
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Call the same named symbol in both libraries and return both outcomes.
pub fn run_both<T: 'static, C: Fn(&Symbol<'static, T>)>(name: &str, call: C) -> (Outcome, Outcome) {
    let l = libs();
    let c_sym: Symbol<'static, T> = unsafe { sym(&l.c, name) };
    let r_sym: Symbol<'static, T> = unsafe { sym(&l.rust, name) };
    let c_out = capture(|| call(&c_sym));
    let r_out = capture(|| call(&r_sym));
    (c_out, r_out)
}

/// Assert the C and Rust exports produce byte-identical stdout and terminate
/// the same way.
pub fn assert_same<T: 'static, C: Fn(&Symbol<'static, T>)>(name: &str, call: C, label: &str) {
    let (c_out, r_out) = run_both::<T, C>(name, call);

    assert_eq!(
        c_out.status,
        0,
        "`{name}` C side terminated abnormally for {label}: {}",
        describe_status(c_out.status)
    );
    assert_eq!(
        c_out.stdout,
        r_out.stdout,
        "`{name}` stdout mismatch for {label}\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&c_out.stdout),
        show(&r_out.stdout)
    );
    assert_eq!(
        describe_status(c_out.status),
        describe_status(r_out.status),
        "`{name}` termination mismatch for {label}"
    );
}
