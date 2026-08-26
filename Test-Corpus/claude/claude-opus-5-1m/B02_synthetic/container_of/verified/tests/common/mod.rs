//! Shared harness for the differential tests.
//!
//! Both the C ground truth and the Rust translation are loaded as **shared
//! objects** through `libloading` and driven only through their exported C ABI
//! symbols. No Rust function is ever called directly, so the `#[no_mangle]`
//! export wrappers are part of what is under test.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CString};
use std::io::Write;
use std::path::{Path, PathBuf};

use libloading::Library;

/// `struct test *find_container_of_a(int *i)` / `..._b`.
pub type FindFn = unsafe extern "C" fn(*mut c_int) -> *mut CTest;

/// `int main(int argc, char **argv)`.
pub type MainFn = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

/// Layout-compatible view of the C `struct test { int a; int b; }`, used to read
/// values back through the pointers the libraries return.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CTest {
    pub a: c_int,
    pub b: c_int,
}

extern "C" {
    /// `fflush(NULL)` flushes every open C output stream — needed because the C
    /// library's `printf` is fully buffered when stdout is a file or a pipe.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    lib: Library,
}

impl Impl {
    pub fn open(name: &'static str, path: PathBuf) -> Impl {
        assert!(
            path.exists(),
            "{name} shared object is missing: {}",
            path.display()
        );
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        Impl { name, path, lib }
    }

    /// Copies an exported symbol out of the library as a plain function pointer.
    /// The `Library` stays alive for as long as `self` does, so the pointer
    /// remains valid.
    fn sym<T: Copy>(&self, name: &[u8]) -> T {
        unsafe {
            *self
                .lib
                .get::<T>(name)
                .unwrap_or_else(|e| panic!("{} does not export {:?}: {e}", self.name, name))
        }
    }

    pub fn find_container_of_a(&self) -> FindFn {
        self.sym(b"find_container_of_a\0")
    }

    pub fn find_container_of_b(&self) -> FindFn {
        self.sym(b"find_container_of_b\0")
    }

    pub fn main(&self) -> MainFn {
        self.sym(b"main\0")
    }

    /// True if the library exports `name`.
    pub fn exports(&self, name: &[u8]) -> bool {
        unsafe { self.lib.get::<*const c_void>(name).is_ok() }
    }
}

/// The C reference built from the untouched `c_src/src/container_of.c` with the
/// flags CMake uses (default build type, `-fPIC`). Produced by `build.rs`.
pub fn c_impl() -> Impl {
    Impl::open("C reference", PathBuf::from(env!("C_SO_PATH")))
}

/// The same C source compiled at `-O2`.
pub fn c_impl_o2() -> Impl {
    Impl::open("C reference (-O2)", PathBuf::from(env!("C_SO_PATH_O2")))
}

/// The Rust translation's `cdylib`, located next to the test binary.
pub fn rust_impl() -> Impl {
    Impl::open("Rust translation", rust_so_path())
}

/// Locates the Rust shared object to test.
///
/// Preference order:
///
/// 1. `target/<profile>/libdriver.so` — the real artifact `cargo build` produces
///    for the `cdylib` target. This is what an external consumer would link
///    against, so it is what should normally be under test.
/// 2. The equivalent object `build.rs` compiles from the same `src/lib.rs`.
///    `cargo test` on its own never builds the `cdylib` (no test target depends
///    on it), and the fallback keeps a bare `cargo test` meaningful instead of
///    failing on a missing file. `./check_all_features.sh` always runs
///    `cargo build` first, so case 1 is what the recorded results come from.
pub fn rust_so_path() -> PathBuf {
    // .../target/<profile>/deps/<test-binary> -> .../target/<profile>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("target/<profile> directory");

    let cargo_artifact = profile_dir.join("libdriver.so");
    if cargo_artifact.exists() {
        return cargo_artifact;
    }

    let fallback = PathBuf::from(env!("RUST_SO_FALLBACK"));
    assert!(
        fallback.exists(),
        "neither {} nor the build.rs fallback {} exists — run `cargo build` first",
        cargo_artifact.display(),
        fallback.display()
    );
    fallback
}

/// Both implementations, plus a convenience label used in assertion messages.
pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

impl Pair {
    pub fn default_config() -> Pair {
        Pair {
            c: c_impl(),
            rust: rust_impl(),
        }
    }

    pub fn o2_config() -> Pair {
        Pair {
            c: c_impl_o2(),
            rust: rust_impl(),
        }
    }
}

// ---------------------------------------------------------------------------
// Reproducible PRNG (xorshift64*), so every row is driven with many inputs.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------------------
// C-style argv construction
// ---------------------------------------------------------------------------

/// Owns the backing storage for a C `argv` array.
pub struct Argv {
    _owned: Vec<CString>,
    ptrs: Vec<*mut c_char>,
}

impl Argv {
    /// Builds `argv` from raw argument bytes (no interior NULs allowed, exactly
    /// like real process arguments). The array is NULL-terminated, and one
    /// extra NULL slot follows it so that reading one past the terminator — as
    /// the C code does when `argc == 1` — touches mapped memory instead of
    /// wandering off, mirroring the `envp` array that follows `argv` in a real
    /// process image.
    pub fn new(args: &[&[u8]]) -> Argv {
        let owned: Vec<CString> = args
            .iter()
            .map(|a| CString::new(a.to_vec()).expect("argument contains an interior NUL"))
            .collect();
        let mut ptrs: Vec<*mut c_char> = owned.iter().map(|s| s.as_ptr() as *mut c_char).collect();
        ptrs.push(std::ptr::null_mut());
        ptrs.push(std::ptr::null_mut());
        Argv { _owned: owned, ptrs }
    }

    /// `argv` with explicit raw entries, allowing NULL or bogus pointers in the
    /// middle for the error-path tests.
    pub fn raw(entries: Vec<*mut c_char>) -> Argv {
        Argv {
            _owned: Vec::new(),
            ptrs: entries,
        }
    }

    pub fn as_ptr(&mut self) -> *mut *mut c_char {
        self.ptrs.as_mut_ptr()
    }

    /// Copies out the current contents of every non-NULL `argv` entry, so a test
    /// can prove a callee did not modify the strings it was handed.
    pub fn snapshot(&self) -> Vec<Vec<u8>> {
        self.ptrs
            .iter()
            .take_while(|p| !p.is_null())
            .map(|&p| unsafe { std::ffi::CStr::from_ptr(p) }.to_bytes().to_vec())
            .collect()
    }

    /// The `argc` a real C runtime would pass (number of entries before the
    /// terminating NULL).
    pub fn argc(&self) -> c_int {
        self.ptrs
            .iter()
            .position(|p| p.is_null())
            .unwrap_or(self.ptrs.len()) as c_int
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Where the captured stdout should be pointed: a regular file (fully buffered
/// by glibc) or a pipe (also fully buffered, but a different `fstat` shape).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sink {
    File,
    Pipe,
}

/// File descriptor 1 is process-wide state, so every redirection — and every
/// `fork()` that depends on it — is serialised. The harness runs tests on
/// several threads by default; without this two tests would swap fd 1 out from
/// under each other. Poisoning is ignored: a panicking test has already been
/// reported, and the lock protects no invariant of its own.
static FD1: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_fd1() -> std::sync::MutexGuard<'static, ()> {
    FD1.lock().unwrap_or_else(|e| e.into_inner())
}

/// Public handle on the fd-1 lock, for tests that do their own `fork()` /
/// redirection instead of going through [`capture_stdout`].
pub fn fd1_guard() -> std::sync::MutexGuard<'static, ()> {
    lock_fd1()
}

/// Runs `f` with file descriptor 1 redirected, then returns everything written.
///
/// Both C `printf` buffers and Rust `std::io::Stdout` buffers are flushed before
/// the descriptor is restored, so the comparison sees the complete output.
pub fn capture_stdout<F: FnOnce()>(sink: Sink, f: F) -> Vec<u8> {
    let _guard = lock_fd1();
    let _ = std::io::stdout().flush();

    unsafe {
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        let (read_fd, write_fd, tmp_path) = match sink {
            Sink::Pipe => {
                let mut fds = [0 as c_int; 2];
                assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe() failed");
                (fds[0], fds[1], None)
            }
            Sink::File => {
                let path = temp_path("capture");
                let cpath = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
                let fd = libc::open(
                    cpath.as_ptr(),
                    libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
                    0o600,
                );
                assert!(fd >= 0, "open({}) failed", path.display());
                (fd, fd, Some(path))
            }
        };

        assert!(libc::dup2(write_fd, 1) >= 0, "dup2 failed");
        if sink == Sink::Pipe {
            // fd 1 is now the only write end; closing this one means the reader
            // observes EOF as soon as fd 1 is restored.
            libc::close(write_fd);
        }

        f();

        // Flush every C stream (the C .so's printf) — the Rust side flushes
        // itself inside its own `main`, but be belt-and-braces about it.
        fflush(std::ptr::null_mut());
        let _ = std::io::stdout().flush();

        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);

        if sink == Sink::File {
            libc::lseek(read_fd, 0, libc::SEEK_SET);
        }

        let data = read_all(read_fd);
        libc::close(read_fd);

        if let Some(path) = tmp_path {
            let _ = std::fs::remove_file(path);
        }

        data
    }
}

unsafe fn read_all(fd: c_int) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = libc::read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
        if n > 0 {
            out.extend_from_slice(&buf[..n as usize]);
        } else {
            break;
        }
    }
    out
}

fn temp_path(tag: &str) -> PathBuf {
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    dir.join(format!("container_of_{tag}_{}_{unique}", std::process::id()))
}

/// Calls an implementation's exported `main` and returns `(return value, stdout)`.
pub fn run_main(imp: &Impl, argc: c_int, argv: &mut Argv, sink: Sink) -> (c_int, Vec<u8>) {
    let entry = imp.main();
    let ptr = argv.as_ptr();
    let mut rc = 0;
    let out = capture_stdout(sink, || {
        rc = unsafe { entry(argc, ptr) };
    });
    (rc, out)
}

/// Runs both implementations on the same arguments and asserts byte equality.
pub fn assert_main_matches(pair: &Pair, args: &[&[u8]], sink: Sink) {
    let mut c_argv = Argv::new(args);
    let mut r_argv = Argv::new(args);
    let argc = c_argv.argc();

    let (c_rc, c_out) = run_main(&pair.c, argc, &mut c_argv, sink);
    let (r_rc, r_out) = run_main(&pair.rust, argc, &mut r_argv, sink);

    assert_eq!(
        c_rc, r_rc,
        "return value mismatch for argv={:?} ({} vs {})",
        pretty(args), pair.c.name, pair.rust.name
    );
    assert_eq!(
        c_out,
        r_out,
        "stdout mismatch for argv={:?}: C={:?} Rust={:?}",
        pretty(args),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
}

/// Human-readable rendering of an argv for assertion messages.
pub fn pretty(args: &[&[u8]]) -> Vec<String> {
    args.iter()
        .map(|a| {
            a.iter()
                .flat_map(|&b| std::ascii::escape_default(b))
                .map(|c| c as char)
                .collect()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Fault isolation: run a call that is expected to die in a forked child so the
// terminating signal can be compared exactly.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Exited(i32),
    Signaled(i32),
}

/// Forks, runs `f` in the child with stdout redirected to a temporary file, and
/// reports how the child terminated together with anything it printed.
///
/// The libraries must already be loaded (and the symbols already resolved) in
/// the parent so the child performs no dynamic-linker work of its own.
pub fn run_in_child<F: FnOnce()>(f: F) -> (Outcome, Vec<u8>) {
    let _guard = lock_fd1();
    let _ = std::io::stdout().flush();

    let path = temp_path("child");
    let cpath = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();

    unsafe {
        let fd = libc::open(
            cpath.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600,
        );
        assert!(fd >= 0, "open({}) failed", path.display());

        let pid = libc::fork();
        assert!(pid >= 0, "fork() failed");

        if pid == 0 {
            // Child: redirect stdout, run the call, flush, leave immediately.
            libc::dup2(fd, 1);
            f();
            fflush(std::ptr::null_mut());
            let _ = std::io::stdout().flush();
            libc::_exit(0);
        }

        let mut status: c_int = 0;
        assert!(libc::waitpid(pid, &mut status, 0) == pid, "waitpid failed");

        libc::lseek(fd, 0, libc::SEEK_SET);
        let out = read_all(fd);
        libc::close(fd);
        let _ = std::fs::remove_file(&path);

        let outcome = if libc::WIFSIGNALED(status) {
            Outcome::Signaled(libc::WTERMSIG(status))
        } else if libc::WIFEXITED(status) {
            Outcome::Exited(libc::WEXITSTATUS(status))
        } else {
            panic!("child neither exited nor was signalled: status={status}");
        };

        (outcome, out)
    }
}
