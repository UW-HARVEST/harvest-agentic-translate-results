//! Shared harness for the C-vs-Rust differential tests.
//!
//! BOTH libraries are loaded as shared objects through `libloading`; no Rust
//! function is ever called directly, so the `#[no_mangle]` export wrappers are
//! exercised exactly as an external C consumer would exercise them.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::ffi::c_int;
use std::path::PathBuf;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture. Declared locally so the test needs no
// extra crates. These resolve against the same glibc that both .so files use,
// therefore `fflush(NULL)` flushes the buffers written by *either* library.
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut core::ffi::c_void) -> c_int;
    fn fork() -> c_int;
    fn _exit(status: c_int) -> !;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
}

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let p = repo_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not built at {p:?}\n\
         build it with: cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

fn rust_so_path() -> PathBuf {
    // Locate libdriver.so next to the test executable's target dir. The test
    // binary lives at target/<profile>/deps/<name>-<hash>, so the cdylib is at
    // target/<profile>/libdriver.so.
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    for _ in 0..4 {
        let cand = dir.join("libdriver.so");
        if cand.exists() {
            return cand;
        }
        match dir.parent() {
            Some(p) => dir = p.to_path_buf(),
            None => break,
        }
    }
    // Fallbacks for an explicit `cargo build` profile dir.
    for prof in ["debug", "release"] {
        let cand = repo_root().join("translation/target").join(prof).join("libdriver.so");
        if cand.exists() {
            return cand;
        }
    }
    panic!("Rust cdylib libdriver.so not found; run `cargo build` first");
}

/// Path to the C shared object (public, for the Phase D symbol tests).
pub fn c_so() -> PathBuf {
    c_so_path()
}

/// Path to the Rust cdylib (public, for the Phase D symbol tests).
pub fn rust_so() -> PathBuf {
    rust_so_path()
}

// ---------------------------------------------------------------------------
// The four exported entry points, as raw C function pointers.
// ---------------------------------------------------------------------------

pub struct Api {
    _lib: Library,
    pub print_line: unsafe extern "C" fn(*const c_char),
    pub bad: unsafe extern "C" fn(),
    pub good: unsafe extern "C" fn(),
    pub driver: unsafe extern "C" fn(c_int),
    pub name: &'static str,
}

impl Api {
    unsafe fn load(path: &PathBuf, name: &'static str) -> Api {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
        let print_line: Symbol<unsafe extern "C" fn(*const c_char)> =
            lib.get(b"printLine\0").expect("printLine");
        let bad: Symbol<unsafe extern "C" fn()> = lib.get(b"bad\0").expect("bad");
        let good: Symbol<unsafe extern "C" fn()> = lib.get(b"good\0").expect("good");
        let driver: Symbol<unsafe extern "C" fn(c_int)> = lib.get(b"driver\0").expect("driver");
        let api = Api {
            print_line: *print_line,
            bad: *bad,
            good: *good,
            driver: *driver,
            name,
            _lib: lib,
        };
        api
    }
}

pub struct Pair {
    pub c: Api,
    pub rust: Api,
}

/// Both libraries, loaded once per test process.
pub fn libs() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| unsafe {
        Pair {
            c: Api::load(&c_so_path(), "C"),
            rust: Api::load(&rust_so_path(), "Rust"),
        }
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// fd 1 is process-global, so every redirection (and every fork) must be
/// serialised or concurrently-running tests stomp on each other's capture.
pub static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run `f` with fd 1 redirected to a temp file and return every byte written
/// to stdout (by the test process *and* by whichever .so `f` calls into).
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let path = PathBuf::from(dir).join(format!(
        "cdiff-{}-{:?}-{}.out",
        std::process::id(),
        std::thread::current().id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));

    let file = std::fs::File::create(&path).expect("create capture file");
    let tmp_fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    unsafe {
        // Flush anything already pending so it is not attributed to us.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmp_fd, 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    drop(file);

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

// ---------------------------------------------------------------------------
// Crash-isolated capture, for the UB (`bad`) path.
//
// The C `bad()` reads an uninitialised `char *` and hands it to `puts`. On this
// toolchain that garbage pointer is un-dereferenceable roughly half the time,
// so the C library SIGSEGVs depending on what the caller left on the stack. To
// observe that without killing the test runner, the call is made in a forked
// child and the child's exit status is reported.
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Terminating signal, if the call crashed (11 == SIGSEGV).
    pub signal: Option<i32>,
    /// Normal exit code, if it returned.
    pub code: Option<i32>,
    /// Bytes the call wrote to stdout before exiting.
    pub out: Vec<u8>,
}

impl Outcome {
    pub fn crashed(&self) -> bool {
        self.signal.is_some()
    }
    /// True when the call returned normally and wrote either nothing or a
    /// complete (newline-terminated) line.
    pub fn clean_line(&self) -> bool {
        !self.crashed() && (self.out.is_empty() || self.out.ends_with(b"\n"))
    }
}

/// Run `f` in a forked child with stdout redirected; report exit status + bytes.
pub fn run_in_child<F: FnOnce()>(f: F) -> Outcome {
    // Shares STDOUT_LOCK with `capture`: a fork must not race a redirection.
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let path = PathBuf::from(dir).join(format!(
        "cdiff-child-{}-{}.out",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let file = std::fs::File::create(&path).expect("create child capture file");
    let tmp_fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    unsafe {
        // Drain our own buffers first so the child cannot duplicate them.
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // --- child: keep this as small as possible ---
            dup2(tmp_fd, 1);
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        let mut status: c_int = 0;
        assert!(waitpid(pid, &mut status, 0) == pid, "waitpid failed");
        drop(file);
        let out = std::fs::read(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);

        // Decode wait(2) status without libc's WIF* macros.
        let sig = status & 0x7f;
        if sig != 0 && sig != 0x7f {
            Outcome { signal: Some(sig), code: None, out }
        } else {
            Outcome { signal: None, code: Some((status >> 8) & 0xff), out }
        }
    }
}

/// Capture C output and Rust output for the same operation and assert they are
/// byte-identical.
pub fn assert_same<F>(what: &str, mut op: F)
where
    F: FnMut(&Api),
{
    let l = libs();
    let c_out = capture(|| op(&l.c));
    let r_out = capture(|| op(&l.rust));
    if c_out != r_out {
        panic!(
            "DIVERGENCE [{what}]\n  C    ({} bytes): {}\n  Rust ({} bytes): {}",
            c_out.len(),
            show(&c_out),
            r_out.len(),
            show(&r_out)
        );
    }
}

pub fn show(b: &[u8]) -> String {
    let mut s = String::from("\"");
    for &x in b.iter().take(400) {
        match x {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(x as char),
            _ => s.push_str(&format!("\\x{x:02x}")),
        }
    }
    s.push('"');
    if b.len() > 400 {
        s.push_str(&format!(" …(+{} more)", b.len() - 400));
    }
    s
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible property tests.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Random byte in `1..=255` (never 0, so it cannot terminate a C string).
    pub fn nonzero_byte(&mut self) -> u8 {
        (self.below(255) + 1) as u8
    }
    /// Random printable ASCII byte.
    pub fn ascii_byte(&mut self) -> u8 {
        (self.below(95) + 32) as u8
    }
}

/// Build a NUL-terminated C string buffer from `body` (which must not contain 0).
pub fn cstr(body: &[u8]) -> Vec<u8> {
    let mut v = body.to_vec();
    v.push(0);
    v
}
