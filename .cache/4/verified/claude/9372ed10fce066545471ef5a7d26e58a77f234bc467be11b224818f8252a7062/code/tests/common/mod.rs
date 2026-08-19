// Shared differential-test harness.
//
// Both the C shared object (built from c_src/) and the Rust shared object
// (target/<profile>/libdriver.so) are loaded with `libloading` and driven
// exclusively through their exported C symbols. Rust functions are NEVER called
// directly from the test crate, so the `#[no_mangle]` / `extern "C"` export
// wrappers are exercised exactly as an external consumer would exercise them.
//
// Both library functions return `void` and communicate only by writing to
// `stdout`, so "compare the outputs" means "capture the bytes each library
// writes to fd 1 and compare them byte-for-byte".

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::io::{Read, Write};
use std::mem;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

/// One loaded implementation, addressed only through raw exported symbols.
pub struct Api {
    pub name: &'static str,
    /// Address of the exported `driver` symbol.
    driver: *const c_void,
    /// Address of the exported `printHexCharLine` symbol.
    print_hex_char_line: *const c_void,
}

// The addresses come from a deliberately leaked `Library`, so they stay valid
// for the whole process lifetime and are safe to share between threads.
unsafe impl Send for Api {}
unsafe impl Sync for Api {}

impl Api {
    /// `void driver(char)` — called with the declared `char` parameter type.
    pub fn driver(&self, v: c_char) {
        let f: unsafe extern "C" fn(c_char) = unsafe { mem::transmute(self.driver) };
        unsafe { f(v) }
    }

    /// `void printHexCharLine(char)` — declared parameter type.
    pub fn print_hex_char_line(&self, v: c_char) {
        let f: unsafe extern "C" fn(c_char) = unsafe { mem::transmute(self.print_hex_char_line) };
        unsafe { f(v) }
    }

    /// `driver` invoked as if its parameter were `int`.
    ///
    /// A C caller may pass any `int` for a `char` parameter (this is the same
    /// ABI situation as passing an out-of-range value for an `enum` parameter):
    /// the argument travels in a 32-bit register and the callee must consider
    /// only the low 8 bits. This deliberately probes that.
    pub fn driver_int(&self, v: c_int) {
        let f: unsafe extern "C" fn(c_int) = unsafe { mem::transmute(self.driver) };
        unsafe { f(v) }
    }

    /// `printHexCharLine` invoked as if its parameter were `int`.
    pub fn print_hex_char_line_int(&self, v: c_int) {
        let f: unsafe extern "C" fn(c_int) = unsafe { mem::transmute(self.print_hex_char_line) };
        unsafe { f(v) }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C reference `.so`. Defaults to the CMake build output, but can be
/// overridden with `DRIVER_C_SO=/path/to/libdriver.so` so the very same
/// differential suite can be run against C builds at other optimisation levels
/// (gcc truncates the `char` argument at every `-O` level, so all of them must
/// agree with the Rust build).
fn c_so_path() -> PathBuf {
    match std::env::var_os("DRIVER_C_SO") {
        Some(p) => PathBuf::from(p),
        None => manifest_dir().join("c_src/build/libdriver.so"),
    }
}

/// `target/<profile>/libdriver.so`, derived from the test executable's own
/// location so it works for both debug and release profiles.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test binary>
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>");
    profile_dir.join("libdriver.so")
}

/// Newest modification time of any file under `dir` (recursively).
fn newest_mtime(dir: &Path) -> Option<std::time::SystemTime> {
    let mut newest: Option<std::time::SystemTime> = None;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(p) = stack.pop() {
        let Ok(entries) = fs::read_dir(&p) else {
            continue;
        };
        for e in entries.flatten() {
            let path = e.path();
            let Ok(md) = e.metadata() else { continue };
            if md.is_dir() {
                // Never descend into build output.
                if path.file_name().map(|n| n == "build").unwrap_or(false) {
                    continue;
                }
                stack.push(path);
            } else if let Ok(t) = md.modified() {
                if newest.map(|n| t > n).unwrap_or(true) {
                    newest = Some(t);
                }
            }
        }
    }
    newest
}

/// Refuse to run against a stale shared object.
///
/// `cargo test` compiles the library for the *test* profile but does **not**
/// relink the `cdylib` artifact, so `target/<profile>/libdriver.so` can easily
/// be older than `src/`. Loading it anyway would make every differential
/// assertion compare the C library against an outdated Rust build — a silent
/// vacuous pass that hides real divergence. Fail loudly instead.
fn assert_fresh(name: &str, so: &Path, sources: &[PathBuf], rebuild_hint: &str) {
    let Ok(so_mtime) = fs::metadata(so).and_then(|m| m.modified()) else {
        return;
    };
    for src in sources {
        if let Some(src_mtime) = newest_mtime(src) {
            assert!(
                src_mtime <= so_mtime,
                "STALE {name} SHARED OBJECT.\n  \
                 {} is older than the sources in {}.\n  \
                 Tests would silently compare against an outdated build.\n  \
                 Rebuild with: {rebuild_hint}",
                so.display(),
                src.display()
            );
        }
    }
}

fn load(name: &'static str, path: PathBuf) -> Api {
    assert!(
        path.exists(),
        "{} shared object not found at {}\n\
         Build both libraries first:\n  \
         (cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .)\n  \
         cargo build",
        name,
        path.display()
    );

    match name {
        "Rust" => assert_fresh(
            name,
            &path,
            &[manifest_dir().join("src")],
            "cargo build   (run it BEFORE cargo test — cargo test does not relink the cdylib)",
        ),
        _ => assert_fresh(
            name,
            &path,
            &[
                manifest_dir().join("c_src/src"),
                manifest_dir().join("c_src/include"),
            ],
            "cmake --build c_src/build",
        ),
    }

    // RTLD_LOCAL is essential. The C `driver` calls `printHexCharLine` through
    // the PLT and the Rust `driver` calls it through the GOT; in both objects
    // the callee is an exported, preemptible symbol. Loading with RTLD_GLOBAL
    // would let whichever library was loaded first interpose its
    // `printHexCharLine` on the other library's `driver`, quietly turning the
    // differential test into a comparison of one implementation with itself.
    // RTLD_LOCAL keeps each object's internal call bound to its own definition.
    let lib = unsafe {
        libloading::os::unix::Library::open(Some(&path), libc::RTLD_NOW | libc::RTLD_LOCAL)
    }
    .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));

    let driver = unsafe {
        *lib.get::<*const c_void>(b"driver\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol `driver`: {e}"))
    };
    let print_hex_char_line = unsafe {
        *lib.get::<*const c_void>(b"printHexCharLine\0")
            .unwrap_or_else(|e| panic!("{name}: missing symbol `printHexCharLine`: {e}"))
    };

    // Keep the library mapped for the rest of the process.
    mem::forget(lib);

    Api {
        name,
        driver,
        print_hex_char_line,
    }
}

pub fn c_api() -> &'static Api {
    static C: OnceLock<Api> = OnceLock::new();
    C.get_or_init(|| load("C", c_so_path()))
}

pub fn rust_api() -> &'static Api {
    static R: OnceLock<Api> = OnceLock::new();
    R.get_or_init(|| load("Rust", rust_so_path()))
}

/// Perform every one-time side effect (dlopen of both objects, allocation of
/// glibc's `stdout` buffer) *before* fd 1 is ever redirected.
///
/// This matters twice over: a lazily-loaded library or a lazily-allocated stdio
/// buffer inside a capture window would contaminate the captured bytes, and a
/// `malloc` inside a forked child could deadlock against the allocator lock.
fn ensure_init() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        // Load both objects.
        let _ = c_api();
        let _ = rust_api();

        // Force glibc to allocate stdout's buffer, discarding the bytes down
        // /dev/null so nothing appears on the real stdout.
        unsafe {
            let devnull = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);
            if devnull >= 0 {
                let saved = libc::dup(1);
                libc::dup2(devnull, 1);
                c_printf(c"prewarm\n".as_ptr());
                libc::fflush(std::ptr::null_mut());
                if saved >= 0 {
                    libc::dup2(saved, 1);
                    libc::close(saved);
                }
                libc::close(devnull);
            }
        }
    });
}

extern "C" {
    #[link_name = "printf"]
    fn c_printf(fmt: *const c_char, ...) -> c_int;

    /// glibc's `stdout` global. The test binary, the C `.so` and the Rust `.so`
    /// all bind to the same `libc.so.6`, so this is the very same `FILE` the
    /// libraries' `printf` calls write into.
    #[link_name = "stdout"]
    static mut C_STDOUT: *mut libc::FILE;
}

// ---------------------------------------------------------------------------
// stdout capture
//
// Every capture runs the library call in a FORKED CHILD whose fd 1 points at a
// temporary file (or a pipe). This is deliberate rather than convenient:
//
// The obvious implementation redirects fd 1 in-process with `dup2`. That is
// subtly WRONG under `cargo test`'s default parallel execution, because libtest
// writes its own progress text ("test c12_... ... ok") to fd 1 from other
// threads. Those writes land inside the redirect window and end up inside the
// captured bytes, producing failures like
//     C   : "test c12_driver_exhaustive_domain ... ffffff81\nok"
//     Rust: "ffffff81\n"
// i.e. spurious diffs that look exactly like translation bugs but are pure
// harness contamination. A mutex cannot fix it: the contaminating writer is
// libtest's own thread, which knows nothing about our lock.
//
// Forking sidesteps it completely: the child gets a private fd 1, the parent's
// fd 1 is never touched, and the child is single-threaded. The suite therefore
// gives identical results under `--test-threads=1` and under full parallelism.
// ---------------------------------------------------------------------------

/// Serialises the one-time init and the fork/wait sequence.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn tmp_path(tag: &str) -> PathBuf {
    let n = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}_{}.out",
        tag,
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ))
}

/// Run `f` in a forked child and return the bytes it wrote to stdout.
///
/// Panics if the child did not exit cleanly, so a crash in either library is
/// reported rather than silently turning into "empty output".
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let res = capture_forked(BufMode::Default, Sink::TempFile, f);
    assert_eq!(
        res.exited_with,
        Some(0),
        "capture child terminated abnormally: {res:?}"
    );
    res.out
}

/// Like [`capture`], but the child's fd 1 is a **pipe** (non-seekable stream)
/// instead of a regular file. Output must stay below the pipe capacity, which
/// every caller respects.
pub fn capture_via_pipe<F: FnOnce()>(f: F) -> Vec<u8> {
    ensure_init();
    let _guard = CAPTURE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let mut fds = [0 as c_int; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
    let (rd, wr) = (fds[0], fds[1]);

    let _ = std::io::stdout().flush();
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        unsafe {
            libc::close(rd);
            if libc::dup2(wr, 1) < 0 {
                libc::_exit(91);
            }
        }
        let code = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(()) => 0,
            Err(_) => 93,
        };
        unsafe {
            libc::fflush(std::ptr::null_mut());
            libc::_exit(code);
        }
    }

    // Parent: close the write end first, or the read below never sees EOF.
    unsafe {
        libc::close(wr);
    }
    let mut bytes = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { libc::read(rd, buf.as_mut_ptr() as *mut c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        bytes.extend_from_slice(&buf[..n as usize]);
    }
    unsafe {
        libc::close(rd);
    }

    let mut status: c_int = 0;
    assert!(
        wait_with_timeout(pid, &mut status),
        "pipe-capture child {pid} did not exit within the timeout"
    );
    assert!(
        libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0,
        "pipe-capture child terminated abnormally (status {status})"
    );
    bytes
}

// ---------------------------------------------------------------------------
// Forked capture (buffering modes and I/O-failure injection)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BufMode {
    /// Leave the inherited buffering alone.
    Default,
    /// `setvbuf(stdout, NULL, _IONBF, 0)`
    Unbuffered,
    /// `setvbuf(stdout, NULL, _IOLBF, 1)`
    LineBuffered,
    /// `setvbuf(stdout, NULL, _IOFBF, 8)` — tiny, to force mid-record flushes.
    FullyBufferedTiny,
}

/// Where fd 1 should point inside the child.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Sink {
    /// A fresh temporary file whose contents are returned.
    TempFile,
    /// `/dev/full` — every `write()` fails with `ENOSPC`.
    DevFull,
    /// A read-only descriptor — every `write()` fails with `EBADF`.
    ReadOnly,
    /// fd 1 closed outright.
    Closed,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ChildResult {
    /// Bytes the child wrote to fd 1 (always empty unless `Sink::TempFile`).
    pub out: Vec<u8>,
    /// `waitpid` status, decoded.
    pub exited_with: Option<c_int>,
    pub signalled_with: Option<c_int>,
}

/// Run `f` in a forked child with `stdout` configured per `mode`/`sink`, and
/// report what the child produced and how it terminated.
///
/// Forking is used for these scenarios so that a poisoned `stdout` error flag,
/// a changed buffering mode, or a crash cannot leak into the rest of the suite.
/// The child performs only `dup2` / `setvbuf` / the library call / `_exit`, and
/// `stdout`'s buffer is pre-allocated by `prewarm()` before any fork, so the
/// child never needs the allocator.
pub fn capture_forked<F: FnOnce()>(mode: BufMode, sink: Sink, f: F) -> ChildResult {
    ensure_init();
    let _guard = CAPTURE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let path = tmp_path("fork");
    let sink_file = match sink {
        Sink::TempFile => Some(
            fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .read(true)
                .write(true)
                .open(&path)
                .expect("create fork capture file"),
        ),
        Sink::DevFull => Some(
            fs::OpenOptions::new()
                .write(true)
                .open("/dev/full")
                .expect("open /dev/full"),
        ),
        // Opened read-only on purpose: writing to it fails with EBADF.
        Sink::ReadOnly => Some(fs::File::open("/dev/null").expect("open /dev/null ro")),
        Sink::Closed => None,
    };
    let sink_fd = sink_file.as_ref().map(|f| f.as_raw_fd());

    // Nothing may be pending in any buffer at fork time, or the child would
    // inherit a copy of it and emit it again.
    let _ = std::io::stdout().flush();
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork() failed");

    if pid == 0 {
        // ---- child: async-signal-safe operations only ----
        unsafe {
            match sink_fd {
                Some(fd) => {
                    if libc::dup2(fd, 1) < 0 {
                        libc::_exit(91);
                    }
                }
                None => {
                    if libc::close(1) < 0 {
                        libc::_exit(92);
                    }
                }
            }
            match mode {
                BufMode::Default => {}
                BufMode::Unbuffered => {
                    libc::setvbuf(stdout_ptr(), std::ptr::null_mut(), libc::_IONBF, 0);
                }
                BufMode::LineBuffered => {
                    libc::setvbuf(stdout_ptr(), std::ptr::null_mut(), libc::_IOLBF, 1);
                }
                BufMode::FullyBufferedTiny => {
                    libc::setvbuf(stdout_ptr(), std::ptr::null_mut(), libc::_IOFBF, 8);
                }
            }
        }

        // A panic must never unwind out of the child, or the child would carry
        // on executing the test harness as a second "parent".
        let code = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(()) => 0,
            Err(_) => 93,
        };

        unsafe {
            libc::fflush(std::ptr::null_mut());
            libc::_exit(code);
        }
    }

    // ---- parent ----
    let mut status: c_int = 0;
    let waited = wait_with_timeout(pid, &mut status);
    assert!(waited, "child {pid} did not exit within the timeout");

    let out = match sink {
        Sink::TempFile => {
            let mut bytes = Vec::new();
            fs::File::open(&path)
                .expect("reopen fork capture file")
                .read_to_end(&mut bytes)
                .expect("read fork capture file");
            bytes
        }
        _ => Vec::new(),
    };
    let _ = fs::remove_file(&path);

    ChildResult {
        out,
        exited_with: if libc::WIFEXITED(status) {
            Some(libc::WEXITSTATUS(status))
        } else {
            None
        },
        signalled_with: if libc::WIFSIGNALED(status) {
            Some(libc::WTERMSIG(status))
        } else {
            None
        },
    }
}

/// glibc's `stdout` is a `*mut FILE` global; take its address without forming a
/// reference to a `static mut`.
fn stdout_ptr() -> *mut libc::FILE {
    unsafe { *std::ptr::addr_of!(C_STDOUT) }
}

/// `waitpid` that cannot hang the test suite: polls for ~10s, then SIGKILLs.
fn wait_with_timeout(pid: libc::pid_t, status: &mut c_int) -> bool {
    for _ in 0..2000 {
        let r = unsafe { libc::waitpid(pid, status, libc::WNOHANG) };
        if r == pid {
            return true;
        }
        if r < 0 {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    unsafe {
        libc::kill(pid, libc::SIGKILL);
        libc::waitpid(pid, status, 0);
    }
    false
}

// ---------------------------------------------------------------------------
// Differential helpers
// ---------------------------------------------------------------------------

/// Run the same closure against the C library and the Rust library, capturing
/// each one's stdout, and assert the two byte streams are identical.
pub fn assert_same<F>(what: &str, mut body: F)
where
    F: FnMut(&'static Api),
{
    let c = capture(|| body(c_api()));
    let r = capture(|| body(rust_api()));
    assert_bytes_eq(what, &c, &r);
}

/// Same as [`assert_same`] but with the Rust library invoked *first*, to rule
/// out any order dependence (lazy PLT binding, first-call stdio setup).
pub fn assert_same_rust_first<F>(what: &str, mut body: F)
where
    F: FnMut(&'static Api),
{
    let r = capture(|| body(rust_api()));
    let c = capture(|| body(c_api()));
    assert_bytes_eq(what, &c, &r);
}

/// Drive `body` over every value in `values` inside a **single** capture per
/// library, then compare the two byte streams.
///
/// Batching matters: each capture costs a `fork`, so running 4096 values as 4096
/// separate captures would mean 8192 forks. Batching keeps it to two, while
/// losing nothing in coverage — the concatenated streams differ if any single
/// value differs.
///
/// To keep the *diagnostics* as precise as per-value captures, a mismatch is
/// localised back to the offending input: `lines_per_value` says how many output
/// lines each input produces, so the first differing line identifies the exact
/// value that diverged, and it is reported along with both libraries' text.
pub fn assert_same_over_values<T, F>(what: &str, values: &[T], lines_per_value: usize, body: F)
where
    T: Copy + std::fmt::Debug,
    F: Fn(&'static Api, T),
{
    let run = |api: &'static Api| {
        for &v in values {
            body(api, v);
        }
    };
    let c = capture(|| run(c_api()));
    let r = capture(|| run(rust_api()));
    if c == r {
        return;
    }

    let c_lines: Vec<&[u8]> = c.split(|&b| b == b'\n').collect();
    let r_lines: Vec<&[u8]> = r.split(|&b| b == b'\n').collect();
    let line_idx = c_lines
        .iter()
        .zip(r_lines.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| c_lines.len().min(r_lines.len()).saturating_sub(1));
    let value_idx = if lines_per_value > 0 {
        line_idx / lines_per_value
    } else {
        0
    };

    let ctx = |lines: &[&[u8]]| -> String {
        let lo = line_idx.saturating_sub(1);
        let hi = (line_idx + 2).min(lines.len());
        lines[lo..hi]
            .iter()
            .map(|l| String::from_utf8_lossy(l).escape_debug().to_string())
            .collect::<Vec<_>>()
            .join(" | ")
    };

    panic!(
        "output mismatch for {what}\n  \
         diverges at output line {line_idx} => input #{value_idx} = {:?}\n  \
         C   line: {}\n  Rust line: {}\n  \
         C   context: {}\n  Rust context: {}\n  \
         total bytes: C {} vs Rust {}",
        values.get(value_idx),
        c_lines
            .get(line_idx)
            .map(|l| String::from_utf8_lossy(l).escape_debug().to_string())
            .unwrap_or_else(|| "<missing>".into()),
        r_lines
            .get(line_idx)
            .map(|l| String::from_utf8_lossy(l).escape_debug().to_string())
            .unwrap_or_else(|| "<missing>".into()),
        ctx(&c_lines),
        ctx(&r_lines),
        c.len(),
        r.len(),
    );
}

pub fn assert_bytes_eq(what: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    panic!(
        "output mismatch for {what}\n  C   ({} bytes): {}\n  Rust({} bytes): {}\n  first diff at byte {}",
        c.len(),
        show(c),
        r.len(),
        show(r),
        c.iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| c.len().min(r.len()))
    );
}

pub fn show(b: &[u8]) -> String {
    let head: Vec<u8> = b.iter().copied().take(400).collect();
    let s = String::from_utf8_lossy(&head).escape_debug().to_string();
    if b.len() > head.len() {
        format!("\"{s}\"... (truncated)")
    } else {
        format!("\"{s}\"")
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5DEE_CE66_D000_1234;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
    pub fn next_i8(&mut self) -> c_char {
        self.next_u8() as c_char
    }
    pub fn next_i32(&mut self) -> c_int {
        (self.next_u64() >> 32) as u32 as c_int
    }
}

/// Every `char` bit pattern, as the signed `c_char` the API declares.
pub fn all_chars() -> impl Iterator<Item = c_char> {
    (0u16..=255).map(|v| v as u8 as c_char)
}

/// The `char` values in the inclusive bit-pattern range `lo..=hi`.
pub fn chars_in(lo: u8, hi: u8) -> impl Iterator<Item = c_char> {
    (lo as u16..=hi as u16).map(|v| v as u8 as c_char)
}
