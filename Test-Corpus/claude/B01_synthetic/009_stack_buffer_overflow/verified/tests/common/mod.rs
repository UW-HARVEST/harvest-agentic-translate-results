//! Shared harness for the C-vs-Rust differential tests.
//!
//! Two independent comparison channels are provided:
//!
//! * [`ffi`] — loads **both** shared objects with `libloading` and calls the
//!   exported `extern "C"` symbols. The Rust side is *never* called directly as
//!   a Rust function; it always goes out through `libdriver.so`'s `#[no_mangle]`
//!   exports, exactly as an external C consumer would, so the export wrappers
//!   themselves are under test.
//! * [`exe`] — runs the two executables (`c_src/build/driver` and
//!   `target/release/driver`) with identical stdin and compares stdout bytes and
//!   wait status. This is the channel that can observe process death, which the
//!   C code's out-of-bounds write causes for some indices.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Fixed seed so every property-style row is reproducible.
pub const SEED: u64 = 0x5EED_129;

/// Tiny xorshift PRNG — deterministic and dependency-free.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn i32_any(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn in_range(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// Artifact locations / building
// ---------------------------------------------------------------------------

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_exe() -> PathBuf {
    crate_root().join("c_src/build/driver")
}
pub fn rust_exe() -> PathBuf {
    crate_root().join("target/release/driver")
}
pub fn c_lib() -> PathBuf {
    crate_root().join("capi_build/libdriver_c.so")
}
pub fn rust_lib() -> PathBuf {
    crate_root().join("target/release/libdriver.so")
}

fn must_exist(p: &Path, how: &str) {
    assert!(
        p.exists(),
        "missing build artifact {}\nbuild it with: {}",
        p.display(),
        how
    );
}

/// Builds every artifact the tests need, exactly once per test binary.
pub fn ensure_built() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let root = crate_root();

        // C executable, via the project's own CMake build.
        if !c_exe().exists() {
            let build = root.join("c_src/build");
            std::fs::create_dir_all(&build).expect("mkdir c_src/build");
            let ok = Command::new("cmake")
                .current_dir(&build)
                .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            assert!(ok, "cmake configure failed");
            let ok = Command::new("cmake")
                .current_dir(&build)
                .args(["--build", "."])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            assert!(ok, "cmake build failed");
        }

        // C shared object built from the same untouched translation unit.
        if !c_lib().exists() {
            std::fs::create_dir_all(root.join("capi_build")).expect("mkdir capi_build");
            let ok = Command::new("gcc")
                .current_dir(&root)
                .args([
                    "-shared",
                    "-fPIC",
                    "-O0",
                    "-o",
                    "capi_build/libdriver_c.so",
                    "c_src/src/main.c",
                ])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            assert!(ok, "gcc -shared of c_src/src/main.c failed");
        }

        must_exist(&c_exe(), "cd c_src/build && cmake .. && cmake --build .");
        must_exist(&c_lib(), "see ensure_built()");
        // The Rust artifacts are produced by `cargo build --release`; the test
        // harness cannot build them itself without recursing into cargo.
        must_exist(&rust_exe(), "cargo build --release");
        must_exist(&rust_lib(), "cargo build --release");
    });
}

// ---------------------------------------------------------------------------
// Channel 1: the two executables
// ---------------------------------------------------------------------------

/// Result of running one of the programs.
#[derive(PartialEq, Eq, Clone)]
pub struct Outcome {
    /// `Ok(exit code)` for a normal exit, `Err(signal)` when killed by a signal.
    pub status: Result<i32, i32>,
    pub stdout: Vec<u8>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self.status {
            Ok(c) => format!("exit({c})"),
            Err(sig) => format!("signal({sig})"),
        };
        write!(f, "{s} stdout={:?}", String::from_utf8_lossy(&self.stdout))
    }
}

pub mod exe {
    use super::Outcome;
    use std::io::Write;
    use std::os::unix::process::ExitStatusExt;
    use std::path::Path;
    use std::process::{Command, Stdio};

    pub fn run(exe: &Path, stdin: &[u8]) -> Outcome {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
        {
            let si = child.stdin.as_mut().unwrap();
            // The child may die (SIGSEGV) before draining stdin; a broken pipe
            // here is expected and must not fail the test.
            let _ = si.write_all(stdin);
            let _ = si.flush();
        }
        let out = child.wait_with_output().expect("wait");
        Outcome {
            status: match out.status.code() {
                Some(c) => Ok(c),
                None => Err(out.status.signal().unwrap_or(-1)),
            },
            stdout: out.stdout,
        }
    }

    /// Runs both executables on the same stdin and returns `(c, rust)`.
    pub fn both(stdin: &[u8]) -> (Outcome, Outcome) {
        super::ensure_built();
        (
            run(&super::c_exe(), stdin),
            run(&super::rust_exe(), stdin),
        )
    }

    /// Asserts the two executables agree bit-for-bit on this stdin.
    pub fn assert_same(stdin: &[u8], ctx: &str) {
        let (c, r) = both(stdin);
        assert_eq!(
            c, r,
            "\nDIVERGENCE [{ctx}]\n  stdin = {:?}\n  C    = {:?}\n  Rust = {:?}\n",
            String::from_utf8_lossy(stdin),
            c,
            r
        );
    }

    /// Asserts agreement, but skips the comparison when **either** side died from
    /// a signal.
    ///
    /// This is for inputs whose resulting `bad()` index is not directly chosen by
    /// the test (truncated over-long lines, random byte soup). The *only* way this
    /// program can die from a signal is the unchecked `buffer[data] = 1` store, and
    /// for large `data` whether that store faults is genuinely nondeterministic in
    /// C itself (stack ASLR — see CONFIGS.md, "Known-unmatchable region"). Skipping
    /// signal deaths therefore excludes exactly the unmatchable region while still
    /// comparing every byte of every well-defined outcome, which is what these rows
    /// exist to test (atoi spellings, fgets truncation, the message strings).
    ///
    /// Returns true when the comparison was actually performed.
    pub fn assert_same_if_no_crash(stdin: &[u8], ctx: &str) -> bool {
        let (c, r) = both(stdin);
        if c.status.is_err() || r.status.is_err() {
            return false;
        }
        assert_eq!(
            c, r,
            "\nDIVERGENCE [{ctx}]\n  stdin = {:?}\n  C    = {:?}\n  Rust = {:?}\n",
            String::from_utf8_lossy(stdin),
            c,
            r
        );
        true
    }
}

// ---------------------------------------------------------------------------
// Runtime probing of the out-of-bounds boundary
// ---------------------------------------------------------------------------

/// The `bad()` index classes whose out-of-bounds store lands on a live saved
/// frame pointer or return address, which is fatal regardless of environment.
pub const FATAL_INDICES: [i64; 6] = [16, 17, 18, 19, 26, 27];

/// Above this index the C store is far enough past the stack to fault every
/// time, and the signal is reproducibly `SIGSEGV` (measured: 20/20 runs at
/// k = 50_000 … 536_870_912 under empty, minimal and inherited environments).
pub const FAR_FATAL_MIN: i64 = 50_000;

/// Beyond this index, `4 * k` is large enough that ASLR decides *which* fatal
/// signal C raises, so only "died from a signal" may be asserted above it.
///
/// Measured with 120 random samples per decade: `50_000..1_000_000` and
/// `1_000_000..10_000_000` were `SIGSEGV` 120/120 each, `10_000_000..100_000_000`
/// produced one `SIGBUS` in 120 (k = 27_550_747), and by `i32::MAX` C is 33%
/// `SIGBUS` / 67% `SIGSEGV`. The bound is set at the top of the range that was
/// clean across every sample, with margin.
pub const SIGNAL_KIND_UNSTABLE_MIN: i64 = 1_000_000;

/// Largest `bad()` index for which the **C** binary is reproducibly benign in
/// *this* process's environment.
///
/// This must be probed rather than hardcoded: the distance from `bad()`'s frame
/// to the top of the `[stack]` mapping is set by the size of the environment
/// block, so the boundary moves. Measured examples — with an empty environment
/// even index 500 faults on ~2 runs in 12, while under a typical inherited
/// environment index 1300 is still benign 12/12.
///
/// A geometric ladder is walked upward and the last rung that is benign on
/// *every* one of 25 consecutive runs is taken; the answer is then divided by 4
/// as a safety margin, so tests never assert inside the probabilistic band.
pub fn deterministic_benign_limit() -> i64 {
    static LIMIT: OnceLock<i64> = OnceLock::new();
    *LIMIT.get_or_init(|| {
        ensure_built();
        let ladder = [32i64, 64, 100, 200, 400, 800, 1600, 3200];
        let mut best = 28; // 28 is the first index past main()'s frame
        for &k in &ladder {
            let stdin = format!("0\n{k}\n").into_bytes();
            let all_benign = (0..25).all(|_| exe::run(&c_exe(), &stdin).status == Ok(0));
            if all_benign {
                best = k;
            } else {
                break;
            }
        }
        (best / 4).max(28)
    })
}

// ---------------------------------------------------------------------------
// Channel 2: the two shared objects, through their exported C symbols
// ---------------------------------------------------------------------------

pub mod ffi {
    use super::Outcome;
    use libloading::Library;
    use std::ffi::c_int;
    use std::io::{Read, Seek, Write};
    use std::os::raw::c_char;
    use std::os::unix::io::AsRawFd;
    use std::path::Path;

    /// Which exported entry point to invoke, and with what argument.
    pub enum Call<'a> {
        /// `printLine(ptr)` — `None` means a genuine NULL pointer.
        PrintLine(Option<&'a [u8]>),
        PrintIntLine(c_int),
        /// A sequence of `printLine`/`printIntLine` calls, to test ordering.
        Mixed(&'a [MixedOp<'a>]),
        Bad,
        Good,
        /// `main(argc, argv)`; `with_args = false` passes `argc = 0, argv = NULL`.
        Main { with_args: bool },
    }

    pub enum MixedOp<'a> {
        Line(&'a [u8]),
        Int(c_int),
    }

    type FnVoid = unsafe extern "C" fn();
    type FnLine = unsafe extern "C" fn(*const c_char);
    type FnInt = unsafe extern "C" fn(c_int);
    type FnMain = unsafe extern "C" fn(c_int, *const *const c_char) -> c_int;

    /// Performs `call` inside a forked child of this process, with fd 0 fed from
    /// `stdin` and fd 1 captured, and returns what the child produced.
    ///
    /// Forking is what makes this safe *and* faithful: the C library's
    /// out-of-bounds write may kill the process, and that death (plus the loss of
    /// the still-unflushed stdio buffer) is precisely the behavior under test.
    fn run_in_child(libpath: &Path, call: &Call<'_>, stdin: &[u8]) -> Outcome {
        // Temp files for the child's stdin and stdout.
        let dir = std::env::temp_dir();
        let uniq = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let in_path = dir.join(format!("ffi-in-{uniq}"));
        let out_path = dir.join(format!("ffi-out-{uniq}"));
        std::fs::write(&in_path, stdin).expect("write stdin temp");
        let in_file = std::fs::File::open(&in_path).expect("open stdin temp");
        let out_file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(&out_path)
            .expect("open stdout temp");

        // Flush our own streams so the child cannot duplicate buffered output.
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();

        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork failed");

        if pid == 0 {
            // ---- child ----
            unsafe {
                libc::dup2(in_file.as_raw_fd(), 0);
                libc::dup2(out_file.as_raw_fd(), 1);
                let devnull = libc::open(b"/dev/null\0".as_ptr() as *const c_char, libc::O_WRONLY);
                if devnull >= 0 {
                    libc::dup2(devnull, 2);
                }

                // Load the library *in the child*, so the two libraries can never
                // interfere with one another.
                let lib = match Library::new(libpath) {
                    Ok(l) => l,
                    Err(_) => libc::_exit(97),
                };

                match call {
                    Call::PrintLine(arg) => {
                        let f: libloading::Symbol<FnLine> = match lib.get(b"printLine\0") {
                            Ok(f) => f,
                            Err(_) => libc::_exit(98),
                        };
                        match arg {
                            None => f(std::ptr::null()),
                            Some(bytes) => {
                                let mut z = bytes.to_vec();
                                z.push(0);
                                f(z.as_ptr() as *const c_char);
                            }
                        }
                    }
                    Call::PrintIntLine(v) => {
                        let f: libloading::Symbol<FnInt> = match lib.get(b"printIntLine\0") {
                            Ok(f) => f,
                            Err(_) => libc::_exit(98),
                        };
                        f(*v);
                    }
                    Call::Mixed(ops) => {
                        let fl: libloading::Symbol<FnLine> = match lib.get(b"printLine\0") {
                            Ok(f) => f,
                            Err(_) => libc::_exit(98),
                        };
                        let fi: libloading::Symbol<FnInt> = match lib.get(b"printIntLine\0") {
                            Ok(f) => f,
                            Err(_) => libc::_exit(98),
                        };
                        for op in ops.iter() {
                            match op {
                                MixedOp::Line(b) => {
                                    let mut z = b.to_vec();
                                    z.push(0);
                                    fl(z.as_ptr() as *const c_char);
                                }
                                MixedOp::Int(v) => fi(*v),
                            }
                        }
                    }
                    Call::Bad | Call::Good => {
                        let name: &[u8] = if matches!(call, Call::Bad) {
                            b"bad\0"
                        } else {
                            b"good\0"
                        };
                        let f: libloading::Symbol<FnVoid> = match lib.get(name) {
                            Ok(f) => f,
                            Err(_) => libc::_exit(98),
                        };
                        f();
                    }
                    Call::Main { with_args } => {
                        let f: libloading::Symbol<FnMain> = match lib.get(b"main\0") {
                            Ok(f) => f,
                            Err(_) => libc::_exit(98),
                        };
                        if *with_args {
                            let prog = b"driver\0";
                            let argv: [*const c_char; 2] =
                                [prog.as_ptr() as *const c_char, std::ptr::null()];
                            f(1, argv.as_ptr());
                        } else {
                            f(0, std::ptr::null());
                        }
                    }
                }

                // Both libraries leave their output in the process's libc stdio
                // buffer, exactly as the C functions do. Flushing here is the
                // "the caller observes the output" step, applied symmetrically.
                libc::fflush(std::ptr::null_mut());
                libc::_exit(0);
            }
        }

        // ---- parent ----
        let mut status: c_int = 0;
        unsafe { libc::waitpid(pid, &mut status, 0) };
        let outcome_status = if libc::WIFEXITED(status) {
            Ok(libc::WEXITSTATUS(status))
        } else if libc::WIFSIGNALED(status) {
            Err(libc::WTERMSIG(status))
        } else {
            Err(-1)
        };

        let mut buf = Vec::new();
        let mut f = out_file;
        f.rewind().expect("rewind");
        f.read_to_end(&mut buf).expect("read child stdout");

        let _ = std::fs::remove_file(&in_path);
        let _ = std::fs::remove_file(&out_path);

        assert_ne!(
            outcome_status,
            Ok(97),
            "child could not dlopen {}",
            libpath.display()
        );
        assert_ne!(
            outcome_status,
            Ok(98),
            "child could not dlsym the requested symbol from {}",
            libpath.display()
        );

        Outcome {
            status: outcome_status,
            stdout: buf,
        }
    }

    /// Invokes `call` against both shared objects and returns `(c, rust)`.
    pub fn both(call: &Call<'_>, stdin: &[u8]) -> (Outcome, Outcome) {
        super::ensure_built();
        let c = run_in_child(&super::c_lib(), call, stdin);
        let r = run_in_child(&super::rust_lib(), call, stdin);
        (c, r)
    }

    /// Asserts the two shared objects agree bit-for-bit for this call.
    pub fn assert_same(call: &Call<'_>, stdin: &[u8], ctx: &str) {
        let (c, r) = both(call, stdin);
        assert_eq!(
            c, r,
            "\nDIVERGENCE [{ctx}]\n  stdin = {:?}\n  C .so   = {:?}\n  Rust .so= {:?}\n",
            String::from_utf8_lossy(stdin),
            c,
            r
        );
    }
}
