// Shared differential-test harness.
//
// Both the C and the Rust implementations are loaded as shared objects through
// `libloading` and driven purely through their exported C symbols. The Rust
// functions are NEVER called directly as Rust code -- that is deliberate, so
// that the `#[no_mangle]` / `extern "C"` export wrappers are exercised exactly
// as an external consumer would exercise them.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* open output streams, including the `stdout`
    /// buffer that the two loaded `.so`s write into.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Fixed seed so every property-style row is reproducible.
pub const SEED: u64 = 0x5EED_1234;

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) -- avoids pulling in an extra dependency.
// ---------------------------------------------------------------------------

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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform-ish in `[0, n)`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    /// Random byte in `lo..=hi`.
    pub fn byte_in(&mut self, lo: u8, hi: u8) -> u8 {
        lo + (self.next_u64() % ((hi - lo) as u64 + 1)) as u8
    }
}

// ---------------------------------------------------------------------------
// Locating / building the shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` -- derived from the test binary's own location
/// (`target/<profile>/deps/<test>-<hash>`), so it is correct for both the debug
/// and release profiles.
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

/// The Rust `cdylib` under test.
///
/// `cargo test` does not necessarily emit the `cdylib` artifact (it builds the
/// lib as a test target), so build it on demand if it is absent. That keeps a
/// bare `cargo test` / `cargo test --release` working without a separate
/// `cargo build` step.
pub fn rust_so_path() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let dir = target_profile_dir();
        let p = dir.join("libdriver.so");
        if !p.exists() {
            let release = dir.file_name().map(|n| n == "release").unwrap_or(false);
            let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
            cmd.arg("build").arg("--offline");
            if release {
                cmd.arg("--release");
            }
            cmd.current_dir(manifest_dir());
            let status = cmd.status().expect("failed to run cargo build");
            assert!(status.success(), "cargo build of the cdylib failed");
        }
        assert!(
            p.exists(),
            "Rust cdylib still not found at {} after `cargo build`",
            p.display()
        );
        p
    })
    .clone()
}

/// The C `.so` built exactly as `c_src/CMakeLists.txt` specifies (no
/// `CMAKE_BUILD_TYPE`, therefore `-O0`). This is the default configuration.
pub fn c_so_path_default() -> PathBuf {
    let p = manifest_dir().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C .so not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// A `-O2` build of the *same, unmodified* C source. Used only for the
/// `bad()` / `driver(0)` rows, where `-O0` is undefined behaviour but every
/// optimising build is well defined (`printLine(NULL)`).
///
/// `c_src/` is never modified: the output goes into `target/`.
pub fn c_so_path_o2() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let out = target_profile_dir().join("libdriver_c_O2.so");
        let src = manifest_dir().join("c_src/src/driver.c");
        let inc = manifest_dir().join("c_src/include");
        let status = Command::new("gcc")
            .args(["-O2", "-fPIC", "-shared"])
            .arg("-I")
            .arg(&inc)
            .arg("-o")
            .arg(&out)
            .arg(&src)
            .status()
            .expect("failed to run gcc");
        assert!(status.success(), "gcc -O2 build of the C source failed");
        out
    })
    .clone()
}

// ---------------------------------------------------------------------------
// The loaded library wrapper
// ---------------------------------------------------------------------------

pub struct Impl {
    lib: Library,
    pub name: &'static str,
}

impl Impl {
    pub fn load(path: &Path, name: &'static str) -> Self {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        Impl { lib, name }
    }

    pub fn print_line(&self, line: *const c_char) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(*const c_char)> =
                self.lib.get(b"printLine\0").expect("printLine");
            f(line)
        }
    }

    pub fn bad(&self) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn()> = self.lib.get(b"bad\0").expect("bad");
            f()
        }
    }

    pub fn good(&self) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn()> = self.lib.get(b"good\0").expect("good");
            f()
        }
    }

    pub fn driver(&self, use_good: c_int) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = self.lib.get(b"driver\0").expect("driver");
            f(use_good)
        }
    }

    /// Raw function pointer for `bad`, resolved eagerly.
    ///
    /// Needed so a caller can `fork()` and invoke it in the child WITHOUT doing
    /// any allocation or `dlsym` after the fork. The `-O0` C `bad()` reads
    /// uninitialized stack residue and can therefore dereference a wild
    /// pointer, so it must be called in a throwaway process.
    pub fn bad_fn_ptr(&self) -> unsafe extern "C" fn() {
        unsafe {
            let f: Symbol<unsafe extern "C" fn()> = self.lib.get(b"bad\0").expect("bad");
            *f
        }
    }

    /// Raw function pointer for `driver`, resolved eagerly (see `bad_fn_ptr`).
    pub fn driver_fn_ptr(&self) -> unsafe extern "C" fn(c_int) {
        unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = self.lib.get(b"driver\0").expect("driver");
            *f
        }
    }

    pub fn has_symbol(&self, sym: &str) -> bool {
        let mut name = sym.as_bytes().to_vec();
        name.push(0);
        unsafe { self.lib.get::<*const c_void>(&name).is_ok() }
    }
}

/// The C implementation built in the default (`-O0`) configuration, plus the
/// Rust implementation. Loaded once per test binary.
pub fn c_default() -> &'static Impl {
    static L: OnceLock<Impl> = OnceLock::new();
    L.get_or_init(|| Impl::load(&c_so_path_default(), "C(-O0, cmake default)"))
}

pub fn c_o2() -> &'static Impl {
    static L: OnceLock<Impl> = OnceLock::new();
    L.get_or_init(|| Impl::load(&c_so_path_o2(), "C(-O2)"))
}

pub fn rust() -> &'static Impl {
    static L: OnceLock<Impl> = OnceLock::new();
    L.get_or_init(|| Impl::load(&rust_so_path(), "Rust"))
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Serializes stdout capture. Redirecting fd 1 with `dup2` is a *process-wide*
/// side effect, so two threads capturing concurrently would steal each other's
/// output. Cargo runs test binaries one at a time but runs the `#[test]`
/// functions inside a binary in parallel, so this lock is what makes the suite
/// correct without requiring `--test-threads=1`.
fn capture_lock() -> &'static std::sync::Mutex<()> {
    static M: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    M.get_or_init(|| std::sync::Mutex::new(()))
}

/// Runs `f` with file descriptor 1 redirected into a temporary file and returns
/// every byte written to it.
///
/// Both `.so`s emit through the process-wide libc `stdout`, so the buffer is
/// flushed before redirecting (to avoid capturing unrelated output) and again
/// before restoring (to force the callee's output into the file).
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::fd::AsRawFd;

    // Held for the whole redirect window. Ignore poisoning: a panicking test
    // body must not disable capture for the remaining tests.
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let mut tmp = tempfile();

    // Drain BOTH buffers that sit in front of fd 1 before stealing it, so no
    // pre-existing output can land in this capture:
    //   * Rust's `std::io::Stdout` LineWriter, and
    //   * libc's `stdout` FILE buffer, which is what the two `.so`s write into.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    unsafe {
        fflush(std::ptr::null_mut());
    }

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(tmp.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }

    if let Err(p) = result {
        std::panic::resume_unwind(p);
    }

    let mut buf = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("seek");
    tmp.read_to_end(&mut buf).expect("read");
    buf
}

/// A unique, immediately-unlinked temporary file.
fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let path = dir.join(format!(
        "driver-difftest-{}-{}-{}.tmp",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap_or_else(|e| panic!("create {}: {e}", path.display()));
    // Unlink immediately; the open handle keeps it alive.
    let _ = std::fs::remove_file(&path);
    f
}

// ---------------------------------------------------------------------------
// Crash-isolated execution (for the -O0 `bad()` undefined behaviour)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
}

/// How a crash-isolated child terminated.
#[derive(Debug, PartialEq, Eq)]
pub enum ChildOutcome {
    Exited(i32),
    Signalled(i32),
}

pub const SIGSEGV: i32 = 11;
pub const SIGBUS: i32 = 7;

/// Calls `f` in a forked child process with stdout pointed at `/dev/null`, and
/// reports how the child terminated.
///
/// This is how the `-O0` C `bad()` is exercised: it reads an uninitialized
/// pointer, so it may return normally *or* segfault depending on stack residue.
/// Running it in a throwaway child means either outcome can be observed and
/// characterised without taking the test process down with it.
///
/// `f` must not allocate or take locks -- resolve every symbol before calling.
pub fn run_isolated<F: Fn()>(f: F) -> ChildOutcome {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");

    if pid == 0 {
        // Child: silence stdout, run the (possibly crashing) operation, exit
        // without running any atexit/destructor machinery.
        unsafe {
            const O_WRONLY: c_int = 1;
            let devnull = open(b"/dev/null\0".as_ptr() as *const c_char, O_WRONLY);
            if devnull >= 0 {
                dup2(devnull, 1);
            }
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
    }

    let mut status: c_int = 0;
    let r = unsafe { waitpid(pid, &mut status as *mut c_int, 0) };
    assert!(r == pid, "waitpid failed");

    // Decode wait status without pulling in the libc crate.
    if status & 0x7f == 0x7f {
        ChildOutcome::Exited(-1) // stopped; not expected here
    } else if status & 0x7f != 0 {
        ChildOutcome::Signalled(status & 0x7f)
    } else {
        ChildOutcome::Exited((status >> 8) & 0xff)
    }
}

// ---------------------------------------------------------------------------
// Differential assertion helpers
// ---------------------------------------------------------------------------

pub fn show(b: &[u8]) -> String {
    let head: Vec<u8> = b.iter().copied().take(96).collect();
    format!(
        "len={} hex={}{}",
        b.len(),
        head.iter().map(|x| format!("{x:02x}")).collect::<String>(),
        if b.len() > 96 { "..(truncated)" } else { "" }
    )
}

/// Runs the same operation against two implementations and asserts the captured
/// stdout bytes are identical.
pub fn assert_same<F>(a: &Impl, b: &Impl, ctx: &str, op: F)
where
    F: Fn(&Impl),
{
    let out_a = capture_stdout(|| op(a));
    let out_b = capture_stdout(|| op(b));
    assert!(
        out_a == out_b,
        "DIVERGENCE [{ctx}]\n  {:>22}: {}\n  {:>22}: {}",
        a.name,
        show(&out_a),
        b.name,
        show(&out_b)
    );
}

/// Convenience: compare the Rust `.so` against the default (`-O0`) C `.so`.
pub fn diff<F>(ctx: &str, op: F)
where
    F: Fn(&Impl),
{
    assert_same(c_default(), rust(), ctx, op);
}

/// Compare the Rust `.so` against the `-O2` C `.so` (used for the UB rows).
pub fn diff_o2<F>(ctx: &str, op: F)
where
    F: Fn(&Impl),
{
    assert_same(c_o2(), rust(), ctx, op);
}

/// Builds a NUL-terminated buffer from `bytes` (which must not contain NUL).
pub fn cstr(bytes: &[u8]) -> Vec<u8> {
    assert!(!bytes.contains(&0), "test payload must not contain NUL");
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

/// What `printLine(s)` is expected to emit: the bytes, then a newline.
pub fn expected_print_line(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(b'\n');
    v
}
