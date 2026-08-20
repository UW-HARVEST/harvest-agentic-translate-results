//! Differential test: `c_src/src/main.c` (ground truth) vs. the Rust
//! translation, compared through the C ABI on both sides.
//!
//! Nothing here calls a Rust function directly.  Every case is executed twice
//! per implementation:
//!
//!   * **`.so` entry point** — this test binary re-execs *itself* as a runner
//!     child which `dlopen`s the library with `libloading`, resolves
//!     `driver_main` **by name**, and calls it through a
//!     `unsafe extern "C" fn() -> c_int` pointer.  The identical harness code
//!     drives `c_ref/libcdriver.so` (built from the untouched C source with
//!     `-Dmain=driver_main`) and `target/<profile>/libdriver.so` (the Rust
//!     `cdylib`), so the `#[no_mangle]` export wrapper is itself on test.
//!
//!   * **executable entry point** — the cmake-built `c_src/build/driver` vs.
//!     `target/<profile>/driver`, which additionally covers libc start-up, the
//!     exit-time stdio flush and the process exit status / terminating signal.
//!
//! `stdout`, `stderr`, the exit code, the terminating signal and the core-dump
//! flag must all match byte-for-byte.
//!
//! Run with `cargo test` / `cargo test --release`.  `harness = false`, so
//! `main()` below is the whole driver.

#![allow(clippy::needless_range_loop)]

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::os::raw::c_int;
use std::os::unix::io::FromRawFd;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ===========================================================================
// libc bits used to build hostile child environments (closed fds, broken
// pipes, inherited signal dispositions).  Applied from `pre_exec`, which the
// Rust standard library runs *after* its own fd/signal normalisation.
// ===========================================================================

#[repr(C)]
#[derive(Clone, Copy)]
struct SigSet([u64; 16]); // glibc's sigset_t is 128 bytes on Linux

#[repr(C)]
#[derive(Clone, Copy)]
struct RLimit {
    rlim_cur: u64,
    rlim_max: u64,
}

extern "C" {
    fn close(fd: c_int) -> c_int;
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn signal(sig: c_int, handler: usize) -> usize;
    fn sigemptyset(set: *mut SigSet) -> c_int;
    fn sigaddset(set: *mut SigSet, sig: c_int) -> c_int;
    fn pthread_sigmask(how: c_int, set: *const SigSet, old: *mut SigSet) -> c_int;
}

const SIGFPE: c_int = 8;
const SIGPIPE: c_int = 13;
const SIG_DFL: usize = 0;
const SIG_IGN: usize = 1;
const SIG_BLOCK: c_int = 0;
const RLIMIT_CORE: c_int = 4;

/// Environment variable that flips this binary into `dlopen` runner mode.
const RUNNER_ENV: &str = "DRIVER_DIFF_RUNNER_SO";
/// When set, the runner calls `driver_main` through a fn pointer declared to
/// take six integer arguments, filled with garbage.  `int main()` declares no
/// parameters, so neither implementation may look at the argument registers.
const RUNNER_ARGS_ENV: &str = "DRIVER_DIFF_RUNNER_GARBAGE_ARGS";

// ===========================================================================
// Runner mode: dlopen a library, resolve `driver_main`, call it.
// ===========================================================================

fn run_as_so_runner(so: PathBuf) -> ! {
    let status = unsafe {
        let lib = match libloading::Library::new(&so) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("runner: dlopen({}) failed: {e}", so.display());
                std::process::exit(120);
            }
        };
        let entry: libloading::Symbol<'_, unsafe extern "C" fn() -> c_int> =
            match lib.get(b"driver_main\0") {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("runner: driver_main not found in {}: {e}", so.display());
                    std::process::exit(121);
                }
            };
        let rc = if std::env::var_os(RUNNER_ARGS_ENV).is_some() {
            // Same symbol, deliberately mis-declared: six garbage integer
            // arguments in rdi/rsi/rdx/rcx/r8/r9.  A parameterless callee must
            // not care.  (Values include out-of-range "enum" bit patterns.)
            let misdeclared: unsafe extern "C" fn(u64, u64, u64, u64, u64, u64) -> c_int =
                std::mem::transmute(*entry);
            misdeclared(
                u64::MAX,
                0xDEAD_BEEF_DEAD_BEEF,
                0x8000_0000_0000_0000,
                0x7FFF_FFFF,
                0xFFFF_FFFF,
                12345,
            )
        } else {
            entry()
        };
        // Deliberately leak the handle: dlclose() would tear down the
        // library's TLS/atexit state before libc's exit-time fflush, which is
        // not what the real program does.  R26 exercises dlclose separately.
        std::mem::forget(lib);
        rc
    };
    // libc's exit() performs the same fflush(stdout) the C program relies on.
    std::process::exit(status);
}

// ===========================================================================
// Deterministic RNG (SplitMix64) -- fixed seed, no external crate.
// ===========================================================================

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_u128(&mut self) -> u128 {
        ((self.next_u64() as u128) << 64) | self.next_u64() as u128
    }
    /// Uniform in `lo..=hi`.
    fn range(&mut self, lo: i128, hi: i128) -> i128 {
        debug_assert!(lo <= hi);
        let span = (hi - lo + 1) as u128;
        lo + (self.next_u128() % span) as i128
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ===========================================================================
// Observable outcome of one run.
// ===========================================================================

#[derive(PartialEq, Eq, Clone, Debug, Default)]
struct Outcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
    core_dumped: bool,
    /// How much of stdin the process left behind for whoever else holds the
    /// same open file description: the final file offset for a seekable stdin,
    /// or the number of bytes still readable for `PipeResidual`.  `None` when
    /// the stdin kind makes it unobservable.
    ///
    /// This catches glibc's block-buffered reads *and* the `lseek` rewind that
    /// `_IO_cleanup()` performs at exit -- behaviour a naive one-byte-at-a-time
    /// Rust `io::stdin()` loop gets wrong in both directions.
    stdin_residual: Option<u64>,
}

impl Outcome {
    fn render(&self) -> String {
        format!(
            "stdout={:?} stderr={:?} code={:?} signal={:?} core={}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr),
            self.code,
            self.signal,
            self.core_dumped
        ) + &format!(" stdin_residual={:?}", self.stdin_residual)
    }
}

// ===========================================================================
// Child-environment knobs (CONFIGS.md axes I, J, K).
// ===========================================================================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StdinKind {
    /// stdin is a regular file holding the case bytes (default).
    File,
    /// stdin is a pipe fed by the parent.
    Pipe,
    /// stdin is /dev/null -- immediate EOF.
    DevNull,
    /// fd 0 is closed before exec -- reads fail with EBADF.
    Closed,
    /// stdin is the read end of a pre-filled, writer-closed pipe (unseekable);
    /// the parent keeps a dup so it can count the bytes left behind.
    PipeResidual,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StdoutKind {
    /// stdout is a pipe read by the parent (default; the only kind whose
    /// bytes can be captured).
    Pipe,
    /// stdout is a regular file, read back afterwards.
    RegularFile,
    DevNull,
    /// /dev/full -- every write fails with ENOSPC.
    DevFull,
    /// fd 1 is closed before exec -- writes fail with EBADF.
    Closed,
    /// stdout is the write end of a pipe whose read end is already closed --
    /// writes raise SIGPIPE / fail with EPIPE.
    BrokenPipe,
}

#[derive(Clone, Copy, Debug)]
struct Env {
    stdin: StdinKind,
    stdout: StdoutKind,
    /// Disposition to install for SIGPIPE just before exec.
    sigpipe: usize,
    sigfpe_ignore: bool,
    sigfpe_block: bool,
    /// Extra command-line arguments.  `int main()` declares no parameters, so
    /// the C program must ignore them entirely -- and so must the Rust one
    /// (its `fn main` never touches `std::env::args`).
    argv_extra: &'static [&'static str],
    /// Call the `.so`'s `driver_main` through a six-integer-argument fn pointer
    /// stuffed with garbage.  Only affects the `.so` entry point.
    garbage_args: bool,
    /// Leave RLIMIT_CORE alone.  Off by default: writing a core file through
    /// systemd-coredump costs ~0.4 s per fatal case, and the flag is compared
    /// on a dedicated row (`CORE`) instead.
    allow_core: bool,
}

impl Default for Env {
    fn default() -> Self {
        Env {
            stdin: StdinKind::File,
            stdout: StdoutKind::Pipe,
            sigpipe: SIG_DFL,
            sigfpe_ignore: false,
            sigfpe_block: false,
            argv_extra: &[],
            garbage_args: false,
            allow_core: false,
        }
    }
}

// ===========================================================================
// Locating and building the four artifacts under test.
// ===========================================================================

struct Ctx {
    /// cmake-built C executable (`c_src/build/driver`).
    c_exe: PathBuf,
    /// cargo-built Rust executable (same profile as this test).
    rust_exe: PathBuf,
    /// `gcc -shared -fPIC -Dmain=driver_main c_src/src/main.c`.
    c_so: PathBuf,
    /// the Rust `cdylib`.
    rust_so: PathBuf,
    /// this test binary, re-execed as the dlopen runner.
    self_exe: PathBuf,
    /// scratch directory.
    tmp: PathBuf,
    /// randomized cases per row.
    cases: usize,
}

fn sh(what: &str, cmd: &mut Command) {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {what}: {e}"));
    if !out.status.success() {
        panic!(
            "{what} failed ({:?})\n--- stdout ---\n{}\n--- stderr ---\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

impl Ctx {
    fn discover() -> Ctx {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let self_exe = std::env::current_exe().expect("current_exe");
        // .../target/<profile>/deps/differential-<hash>  ->  .../target/<profile>
        let profile_dir = self_exe
            .parent()
            .and_then(Path::parent)
            .expect("profile dir")
            .to_path_buf();

        let rust_exe = PathBuf::from(env!("CARGO_BIN_EXE_driver"));
        let rust_so = profile_dir.join("libdriver.so");

        // `cargo test` builds the lib only as an rlib -- it does NOT emit the
        // cdylib.  Without this step the harness would happily test a stale
        // `libdriver.so` from an earlier build and report a false PASS (a real
        // bug this harness had, caught by mutation testing).
        let profile_name = profile_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("debug")
            .to_string();
        let mut build = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
        build.current_dir(&manifest).args(["build", "--offline", "--lib"]);
        if profile_name == "release" {
            build.arg("--release");
        }
        match build.output() {
            Ok(o) if o.status.success() => {}
            Ok(o) => panic!(
                "`cargo build --lib` (needed to refresh the cdylib) failed:\n{}",
                String::from_utf8_lossy(&o.stderr)
            ),
            Err(e) => panic!("could not run cargo to refresh the cdylib: {e}"),
        }

        assert!(
            rust_so.exists(),
            "missing Rust cdylib at {} -- is crate-type = [\"rlib\", \"cdylib\"] set?",
            rust_so.display()
        );
        assert!(
            rust_exe.exists(),
            "missing Rust executable at {}",
            rust_exe.display()
        );
        // Belt and braces: refuse to run against a cdylib older than its source.
        for src in ["src/lib.rs", "src/main.rs"] {
            let src = manifest.join(src);
            if let (Ok(a), Ok(b)) = (fs::metadata(&src), fs::metadata(&rust_so)) {
                if let (Ok(a), Ok(b)) = (a.modified(), b.modified()) {
                    assert!(
                        a <= b,
                        "{} is newer than {} -- run `cargo build{}` first",
                        src.display(),
                        rust_so.display(),
                        if profile_name == "release" { " --release" } else { "" }
                    );
                }
            }
        }
        for src in ["src/main.rs", "src/lib.rs"] {
            let src = manifest.join(src);
            if let (Ok(a), Ok(b)) = (fs::metadata(&src), fs::metadata(&rust_exe)) {
                if let (Ok(a), Ok(b)) = (a.modified(), b.modified()) {
                    assert!(
                        a <= b,
                        "{} is newer than the built executable {} -- run `cargo build` first",
                        src.display(),
                        rust_exe.display()
                    );
                }
            }
        }

        let tmp = profile_dir.join("diff-tmp");
        fs::create_dir_all(&tmp).expect("create scratch dir");

        let c_src = manifest.join("c_src");
        let c_main = c_src.join("src/main.c");
        assert!(c_main.exists(), "missing {}", c_main.display());

        // --- C executable: the exact build the task prescribes (cmake). ------
        let c_build = c_src.join("build");
        let c_exe = c_build.join("driver");
        if !c_exe.exists() {
            fs::create_dir_all(&c_build).expect("create c_src/build");
            sh(
                "cmake configure",
                Command::new("cmake")
                    .current_dir(&c_build)
                    .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"]),
            );
            sh(
                "cmake build",
                Command::new("cmake").current_dir(&c_build).args(["--build", "."]),
            );
        }
        assert!(c_exe.exists(), "cmake did not produce {}", c_exe.display());

        // --- C shared library ------------------------------------------------
        // `-Dmain=driver_main` is a *compiler flag*; nothing in c_src/ is
        // modified.  No -O flag, mirroring CMake's empty CMAKE_C_FLAGS.
        let c_so_dir = profile_dir.join("c_ref");
        fs::create_dir_all(&c_so_dir).expect("create c_ref dir");
        let c_so = c_so_dir.join("libcdriver.so");
        let needs_build = match (fs::metadata(&c_so), fs::metadata(&c_main)) {
            (Ok(a), Ok(b)) => match (a.modified(), b.modified()) {
                (Ok(a), Ok(b)) => a < b,
                _ => true,
            },
            _ => true,
        };
        if needs_build {
            sh(
                "gcc -shared",
                Command::new("gcc").args([
                    "-shared",
                    "-fPIC",
                    "-Dmain=driver_main",
                    c_main.to_str().unwrap(),
                    "-o",
                    c_so.to_str().unwrap(),
                ]),
            );
        }
        assert!(c_so.exists(), "gcc did not produce {}", c_so.display());

        let cases = std::env::var("DRIVER_DIFF_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(120usize);

        Ctx {
            c_exe,
            rust_exe,
            c_so,
            rust_so,
            self_exe,
            tmp,
            cases,
        }
    }

    // -----------------------------------------------------------------------
    // Running one side.
    // -----------------------------------------------------------------------

    fn run(&self, exe: &Path, so: Option<&Path>, input: &[u8], env: &Env) -> Outcome {
        // Rows run concurrently, so every call needs its own scratch files.
        static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let stdin_path = self.tmp.join(format!("stdin-{seq}.bin"));
        let stdout_path = self.tmp.join(format!("stdout-{seq}.bin"));

        let mut cmd = Command::new(exe);
        cmd.args(env.argv_extra);
        cmd.env_remove(RUNNER_ENV);
        cmd.env_remove(RUNNER_ARGS_ENV);
        if let Some(so) = so {
            cmd.env(RUNNER_ENV, so);
            if env.garbage_args {
                cmd.env(RUNNER_ARGS_ENV, "1");
            }
        }

        // ---- stdin ----------------------------------------------------------
        // For the observable kinds the parent keeps a dup of the very same open
        // file description, so it can see where the child left the offset.
        let mut stdin_probe: Option<File> = None;
        match env.stdin {
            StdinKind::File => {
                fs::write(&stdin_path, input).expect("write stdin file");
                let f = File::open(&stdin_path).expect("open stdin file");
                // try_clone() is dup(2): same file description, shared offset.
                stdin_probe = Some(f.try_clone().expect("dup stdin"));
                cmd.stdin(Stdio::from(f));
            }
            StdinKind::Pipe => {
                cmd.stdin(Stdio::piped());
            }
            StdinKind::PipeResidual => {
                assert!(
                    input.len() < 60 * 1024,
                    "PipeResidual needs the input to fit in the pipe buffer"
                );
                let mut fds = [0 as c_int; 2];
                let (read_end, write_end) = unsafe {
                    assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe() failed");
                    (File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1]))
                };
                {
                    let mut w = write_end;
                    w.write_all(input).expect("prefill pipe");
                } // dropped => the pipe has no writers left, so reads see EOF
                stdin_probe = Some(read_end.try_clone().expect("dup pipe read end"));
                cmd.stdin(Stdio::from(read_end));
            }
            StdinKind::DevNull | StdinKind::Closed => {
                cmd.stdin(Stdio::null());
            }
        }

        // ---- stdout ---------------------------------------------------------
        let mut broken_pipe_fd = None;
        match env.stdout {
            StdoutKind::Pipe => {
                cmd.stdout(Stdio::piped());
            }
            StdoutKind::RegularFile => {
                let f = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&stdout_path)
                    .expect("create stdout file");
                cmd.stdout(Stdio::from(f));
            }
            StdoutKind::DevNull | StdoutKind::Closed => {
                cmd.stdout(Stdio::null());
            }
            StdoutKind::DevFull => {
                let f = OpenOptions::new()
                    .write(true)
                    .open("/dev/full")
                    .expect("open /dev/full");
                cmd.stdout(Stdio::from(f));
            }
            StdoutKind::BrokenPipe => {
                let mut fds = [0 as c_int; 2];
                unsafe {
                    assert_eq!(pipe(fds.as_mut_ptr()), 0, "pipe() failed");
                    // Close the read end *now*, before the child exists, so the
                    // very first write is guaranteed to hit EPIPE/SIGPIPE.
                    assert_eq!(close(fds[0]), 0, "close(read end) failed");
                    cmd.stdout(Stdio::from_raw_fd(fds[1]));
                }
                broken_pipe_fd = Some(fds[1]);
            }
        }
        cmd.stderr(Stdio::piped());

        // ---- child-side fd / signal surgery ---------------------------------
        let stdin_kind = env.stdin;
        let stdout_kind = env.stdout;
        let sigpipe = env.sigpipe;
        let sigfpe_ignore = env.sigfpe_ignore;
        let sigfpe_block = env.sigfpe_block;
        let allow_core = env.allow_core;
        unsafe {
            cmd.pre_exec(move || {
                if !allow_core {
                    // Identical on both sides, so the differential stays valid.
                    let rl = RLimit {
                        rlim_cur: 0,
                        rlim_max: 0,
                    };
                    setrlimit(RLIMIT_CORE, &rl);
                }
                if stdin_kind == StdinKind::Closed {
                    close(0);
                }
                if stdout_kind == StdoutKind::Closed {
                    close(1);
                }
                signal(SIGPIPE, sigpipe);
                if sigfpe_ignore {
                    signal(SIGFPE, SIG_IGN);
                }
                if sigfpe_block {
                    let mut set = SigSet([0u64; 16]);
                    sigemptyset(&mut set);
                    sigaddset(&mut set, SIGFPE);
                    pthread_sigmask(SIG_BLOCK, &set, std::ptr::null_mut());
                }
                Ok(())
            });
        }

        let mut child = cmd
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));

        if env.stdin == StdinKind::Pipe {
            // Only used for inputs comfortably below the 64 KiB pipe buffer.
            let mut si = child.stdin.take().expect("child stdin");
            let _ = si.write_all(input);
            drop(si);
        }

        let out = child.wait_with_output().expect("wait_with_output");

        if let Some(fd) = broken_pipe_fd {
            // `Stdio::from_raw_fd` took ownership and the parent's duplicate was
            // closed by `spawn`; nothing left to do but make sure.
            let _ = fd;
        }

        let stdout = match env.stdout {
            StdoutKind::Pipe => out.stdout,
            StdoutKind::RegularFile => fs::read(&stdout_path).unwrap_or_default(),
            // Not observable; must stay identical by virtue of the status.
            _ => Vec::new(),
        };
        let stdin_residual = match (env.stdin, stdin_probe) {
            (StdinKind::File, Some(mut f)) => f.stream_position().ok(),
            (StdinKind::PipeResidual, Some(mut f)) => {
                let mut rest = Vec::new();
                f.read_to_end(&mut rest).ok().map(|_| rest.len() as u64)
            }
            _ => None,
        };

        let _ = fs::remove_file(&stdin_path);
        let _ = fs::remove_file(&stdout_path);

        Outcome {
            stdout,
            stderr: out.stderr,
            code: out.status.code(),
            signal: out.status.signal(),
            core_dumped: out.status.core_dumped(),
            stdin_residual,
        }
    }

    fn run_c_exe(&self, input: &[u8], env: &Env) -> Outcome {
        self.run(&self.c_exe, None, input, env)
    }
    fn run_rust_exe(&self, input: &[u8], env: &Env) -> Outcome {
        self.run(&self.rust_exe, None, input, env)
    }
    fn run_c_so(&self, input: &[u8], env: &Env) -> Outcome {
        let so = self.c_so.clone();
        self.run(&self.self_exe, Some(&so), input, env)
    }
    fn run_rust_so(&self, input: &[u8], env: &Env) -> Outcome {
        let so = self.rust_so.clone();
        self.run(&self.self_exe, Some(&so), input, env)
    }

    // -----------------------------------------------------------------------
    // Comparison helpers.  Every one returns Err(details) on divergence.
    // -----------------------------------------------------------------------

    /// Compare through the `.so`/`driver_main` C-ABI entry point only.
    fn cmp_so(&self, input: &[u8], env: &Env) -> Result<(), String> {
        let c = self.run_c_so(input, env);
        let r = self.run_rust_so(input, env);
        if c == r {
            Ok(())
        } else {
            Err(format!(
                "[.so/driver_main] input={}\n      C:    {}\n      Rust: {}",
                pretty(input),
                c.render(),
                r.render()
            ))
        }
    }

    /// Compare through the built executables only.
    fn cmp_exe(&self, input: &[u8], env: &Env) -> Result<(), String> {
        let c = self.run_c_exe(input, env);
        let r = self.run_rust_exe(input, env);
        if c == r {
            Ok(())
        } else {
            Err(format!(
                "[exe env={:?}] input={}\n      C:    {}\n      Rust: {}",
                env,
                pretty(input),
                c.render(),
                r.render()
            ))
        }
    }

    /// Run all four artifacts once each (4 spawns, no redundancy).
    fn all_four(&self, input: &[u8], env: &Env) -> Quad {
        Quad {
            c_so: self.run_c_so(input, env),
            rust_so: self.run_rust_so(input, env),
            c_exe: self.run_c_exe(input, env),
            rust_exe: self.run_rust_exe(input, env),
        }
    }

    /// Compare through BOTH entry points (CONFIGS.md column *L*), and also
    /// assert that the two entry points agree with each other.
    fn cmp_both(&self, input: &[u8]) -> Result<(), String> {
        self.all_four(input, &Env::default()).verify(input)
    }
}

/// The four measurements of one case: `{C, Rust} x {.so entry point, exe}`.
struct Quad {
    c_so: Outcome,
    rust_so: Outcome,
    c_exe: Outcome,
    rust_exe: Outcome,
}

impl Quad {
    /// C == Rust through the `.so`, C == Rust through the exe, and the two
    /// entry points agree with each other.
    fn verify(&self, input: &[u8]) -> Result<(), String> {
        if self.c_so != self.rust_so {
            return Err(format!(
                "[.so/driver_main] input={}\n      C:    {}\n      Rust: {}",
                pretty(input),
                self.c_so.render(),
                self.rust_so.render()
            ));
        }
        if self.c_exe != self.rust_exe {
            return Err(format!(
                "[exe] input={}\n      C:    {}\n      Rust: {}",
                pretty(input),
                self.c_exe.render(),
                self.rust_exe.render()
            ));
        }
        // The exe is just `exit(driver_main())`, so the two entry points must
        // produce the same bytes and die the same way.
        if self.c_so.stdout != self.c_exe.stdout || self.c_so.signal != self.c_exe.signal {
            return Err(format!(
                "C .so and C exe disagree for input={}\n      so:  {}\n      exe: {}",
                pretty(input),
                self.c_so.render(),
                self.c_exe.render()
            ));
        }
        if self.rust_so.stdout != self.rust_exe.stdout
            || self.rust_so.signal != self.rust_exe.signal
        {
            return Err(format!(
                "Rust .so and Rust exe disagree for input={}\n      so:  {}\n      exe: {}",
                pretty(input),
                self.rust_so.render(),
                self.rust_exe.render()
            ));
        }
        Ok(())
    }
}

fn pretty(input: &[u8]) -> String {
    if input.len() > 96 {
        format!(
            "{:?}...<{} bytes total>",
            String::from_utf8_lossy(&input[..96]),
            input.len()
        )
    } else {
        format!("{:?}", String::from_utf8_lossy(input))
    }
}

// ===========================================================================
// Input generators
// ===========================================================================

const SEED: u64 = 20_260_818;

const WS: [&str; 6] = [" ", "\t", "\n", "\u{b}", "\u{c}", "\r"];

fn ws_run(rng: &mut Rng, min: usize, max: usize) -> String {
    let n = min + rng.below(max - min + 1);
    (0..n).map(|_| *rng.pick(&WS)).collect()
}

fn digits(rng: &mut Rng, n: usize) -> String {
    (0..n)
        .map(|_| (b'0' + (rng.below(10) as u8)) as char)
        .collect()
}

/// `"<x><sep><y>"` with a single space separator.
fn pair(x: i128, y: i128) -> Vec<u8> {
    format!("{x} {y}").into_bytes()
}

const I32_BOUNDARY: [i64; 9] = [
    i32::MIN as i64,
    i32::MIN as i64 + 1,
    -2,
    -1,
    0,
    1,
    2,
    i32::MAX as i64 - 1,
    i32::MAX as i64,
];

// ===========================================================================
// Phase B -- CONFIGS.md rows
// ===========================================================================

fn r1_canonical_full_range(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 1);
    let mut n = 0;
    for _ in 0..ctx.cases {
        let x = rng.range(i32::MIN as i128, i32::MAX as i128);
        let mut y = rng.range(i32::MIN as i128, i32::MAX as i128);
        if y == 0 {
            y = 1;
        }
        if x == i32::MIN as i128 && y == -1 {
            y = -2;
        }
        ctx.cmp_both(&pair(x, y))?;
        n += 1;
    }
    Ok(n)
}

fn r2_small_all_quadrants(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for x in -20i128..=20 {
        for y in -20i128..=20 {
            if y == 0 {
                continue;
            }
            ctx.cmp_so(&pair(x, y), &Env::default())?;
            n += 1;
        }
    }
    // The composed pipeline (executable) over a representative slice.
    for x in [-20i128, -7, -1, 0, 1, 7, 20] {
        for y in [-20i128, -7, -3, -1, 1, 3, 7, 20] {
            ctx.cmp_exe(&pair(x, y), &Env::default())?;
            n += 1;
        }
    }
    Ok(n)
}

fn r3_exact_division(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 3);
    let mut n = 0;
    for _ in 0..ctx.cases {
        let y = loop {
            let v = rng.range(-10_000, 10_000);
            if v != 0 {
                break v;
            }
        };
        let k = rng.range(-2000, 2000);
        let x = (k * y) as i64 as i32 as i128; // keep it inside int range
        ctx.cmp_both(&pair(x, y))?;
        n += 1;
    }
    Ok(n)
}

fn r4_dividend_smaller(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 4);
    let mut n = 0;
    for _ in 0..ctx.cases {
        let y = loop {
            let v = rng.range(2, i32::MAX as i128);
            if v != 0 {
                break v;
            }
        };
        let y = if rng.bool() { -y } else { y };
        let x = rng.range(-(y.abs() - 1), y.abs() - 1);
        ctx.cmp_both(&pair(x, y))?;
        n += 1;
    }
    Ok(n)
}

fn r5_zero_and_unit_operands(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 5);
    let mut n = 0;
    for _ in 0..ctx.cases {
        let y = loop {
            let v = rng.range(i32::MIN as i128, i32::MAX as i128);
            if v != 0 {
                break v;
            }
        };
        ctx.cmp_both(&pair(0, y))?;
        n += 1;
    }
    for _ in 0..ctx.cases {
        let x = rng.range(i32::MIN as i128 + 1, i32::MAX as i128);
        ctx.cmp_both(&pair(x, 1))?;
        ctx.cmp_both(&pair(x, -1))?;
        n += 2;
    }
    // x == y, and y == x (self-division) for a few magnitudes.
    for v in [1i128, -1, 7, -7, 65_536, i32::MAX as i128, i32::MIN as i128] {
        ctx.cmp_both(&pair(v, v))?;
        n += 1;
    }
    Ok(n)
}

fn r6_int_boundary_cross_product(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for &x in I32_BOUNDARY.iter() {
        for &y in I32_BOUNDARY.iter() {
            if y == 0 {
                continue; // E16
            }
            if x == i32::MIN as i64 && y == -1 {
                continue; // E17
            }
            ctx.cmp_both(&pair(x as i128, y as i128))?;
            n += 1;
        }
    }
    Ok(n)
}

fn r7_whitespace_matrix(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 7);
    let mut n = 0;
    for _ in 0..ctx.cases {
        let lead = ws_run(&mut rng, 0, 5);
        let mid = ws_run(&mut rng, 1, 5);
        let trail = ws_run(&mut rng, 0, 5);
        let x = rng.range(-9999, 9999);
        let y = loop {
            let v = rng.range(-999, 999);
            if v != 0 {
                break v;
            }
        };
        let s = format!("{lead}{x}{mid}{y}{trail}");
        ctx.cmp_both(s.as_bytes())?;
        n += 1;
    }
    // Every single whitespace byte in every position, exhaustively.
    for a in WS.iter() {
        for b in WS.iter() {
            for c in WS.iter() {
                ctx.cmp_so(format!("{a}17{b}5{c}").as_bytes(), &Env::default())?;
                n += 1;
            }
        }
    }
    Ok(n)
}

fn r8_explicit_signs(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 8);
    let signs = ["", "+", "-"];
    let mut n = 0;
    for sx in signs.iter() {
        for sy in signs.iter() {
            for _ in 0..12 {
                let xm = rng.range(0, 99_999);
                let ym = loop {
                    let v = rng.range(0, 9_999);
                    if v != 0 {
                        break v;
                    }
                };
                ctx.cmp_both(format!("{sx}{xm} {sy}{ym}").as_bytes())?;
                n += 1;
            }
        }
    }
    // Signed zeros: "-0"/"+0" as x is valid; as y it traps (covered by E16).
    for s in ["-0 5", "+0 5", "-0 -5", "+0 -5", "-0 1", "+0 1"] {
        ctx.cmp_both(s.as_bytes())?;
        n += 1;
    }
    Ok(n)
}

fn r9_leading_zeros(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 9);
    let mut n = 0;
    for _ in 0..ctx.cases {
        let zx = "0".repeat(rng.below(41));
        let zy = "0".repeat(rng.below(41));
        let sx = if rng.bool() { "-" } else { "" };
        let x = rng.range(0, 30_000);
        let y = loop {
            let v = rng.range(0, 3_000);
            if v != 0 {
                break v;
            }
        };
        ctx.cmp_both(format!("{sx}{zx}{x} {zy}{y}").as_bytes())?;
        n += 1;
    }
    Ok(n)
}

fn r10_long_to_int_truncation(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 10);
    let lim = 1i128 << 33;
    let mut n = 0;
    for _ in 0..ctx.cases {
        let x = rng.range(-lim, lim);
        let y = rng.range(-lim, lim);
        ctx.cmp_both(&pair(x, y))?;
        n += 1;
    }
    // Exact truncation boundaries.
    for v in [
        2_147_483_648i128,
        2_147_483_649,
        -2_147_483_649,
        -2_147_483_650,
        4_294_967_295,
        4_294_967_296,
        4_294_967_297,
        -4_294_967_296,
        8_589_934_592,
    ] {
        ctx.cmp_both(&pair(v, 3))?;
        ctx.cmp_both(&pair(3, v))?;
        n += 2;
    }
    Ok(n)
}

fn r11_long_range_clamping(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 11);
    let lo = -(1i128 << 63) - 8;
    let hi = (1i128 << 63) + 8;
    let mut n = 0;
    for _ in 0..ctx.cases {
        let x = rng.range(lo, hi);
        let y = rng.range(lo, hi);
        ctx.cmp_both(&pair(x, y))?;
        n += 1;
    }
    for v in [
        9_223_372_036_854_775_806i128,
        9_223_372_036_854_775_807,
        9_223_372_036_854_775_808,
        9_223_372_036_854_775_809,
        -9_223_372_036_854_775_807,
        -9_223_372_036_854_775_808,
        -9_223_372_036_854_775_809,
        -9_223_372_036_854_775_810,
        18_446_744_073_709_551_615,
        18_446_744_073_709_551_616,
        18_446_744_073_709_551_617,
    ] {
        ctx.cmp_both(&pair(v, 3))?;
        ctx.cmp_both(&pair(3, v))?;
        n += 2;
    }
    Ok(n)
}

fn r12_very_long_digit_strings(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 12);
    let mut n = 0;
    for len in [100usize, 101, 200, 512, 1000, 2048, 5000] {
        for _ in 0..3 {
            let sx = if rng.bool() { "-" } else { "" };
            let dx = digits(&mut rng, len);
            let y = rng.range(1, 9);
            ctx.cmp_both(format!("{sx}{dx} {y}").as_bytes())?;
            n += 1;
        }
        // long string in the *second* position too
        let dy = digits(&mut rng, len);
        ctx.cmp_both(format!("7 {dy}").as_bytes())?;
        ctx.cmp_both(format!("7 -{dy}").as_bytes())?;
        n += 2;
        // all-zeros of that length is in-range, not clamped
        ctx.cmp_both(format!("{} 4", "0".repeat(len)).as_bytes())?;
        n += 1;
    }
    Ok(n)
}

fn r13_single_token(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 13);
    let tails = ["", " ", "\n", "\t", "  \n\t ", "q", ".", ",", "-", "+", "\0", "\u{ff}"];
    let mut n = 0;
    for tail in tails.iter() {
        for _ in 0..6 {
            let x = rng.range(i32::MIN as i128, i32::MAX as i128);
            let mut s = format!("{x}").into_bytes();
            s.extend_from_slice(tail.as_bytes());
            ctx.cmp_both(&s)?;
            n += 1;
        }
    }
    Ok(n)
}

fn r14_more_than_two_tokens(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 14);
    let mut n = 0;
    for _ in 0..ctx.cases {
        let count = 3 + rng.below(4);
        let mut parts: Vec<String> = Vec::new();
        for i in 0..count {
            let v = rng.range(-9999, 9999);
            // keep the 2nd token non-zero so the row stays on the valid path
            let v = if i == 1 && v == 0 { 5 } else { v };
            parts.push(format!("{v}"));
        }
        let sep = ws_run(&mut rng, 1, 3);
        ctx.cmp_both(parts.join(&sep).as_bytes())?;
        n += 1;
    }
    Ok(n)
}

fn r15_base_prefix_shapes(ctx: &Ctx) -> Result<usize, String> {
    let shapes = [
        "0", "00", "000", "0 0", "0 5", "5 0x2", "0x10", "0X10", "0x", "0X", "010 3", "0o7 3",
        "0b1 3", "0e1 3", "00x10 3", "0x10 0x20", "7 0x3", "7 010", "-0x10 3", "+0x10 3", "0xg 3",
        "0 0x0",
    ];
    let mut n = 0;
    for s in shapes.iter() {
        ctx.cmp_both(s.as_bytes())?;
        n += 1;
    }
    Ok(n)
}

fn r16_nondigit_terminators(ctx: &Ctx) -> Result<usize, String> {
    let shapes = [
        "5.5 2", "5e3 2", "5E3 2", "12,34", "7q 3", "1-2", "3+4", "9/2", "5:2", "8;3", "4)5",
        "6_2", "1'000 3", "-5.5 -2", "5. 2", ".5 2", "5..5 2", "3 4.5", "3 4e2", "3 4,5", "3 4-5",
        "1e 2", "5 2.", "0. 1", "5\u{7f}2",
    ];
    let mut n = 0;
    for s in shapes.iter() {
        ctx.cmp_both(s.as_bytes())?;
        n += 1;
    }
    Ok(n)
}

fn r17_nul_and_high_byte_terminators(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for term in [0u8, 0x01, 0x7f, 0x80, 0xa0, 0xc3, 0xff] {
        cases.push([b"12".as_slice(), &[term], b" 5"].concat());
        cases.push([b"12 5".as_slice(), &[term]].concat());
        cases.push([b"12 ".as_slice(), &[term], b"5"].concat());
        cases.push([&[term][..], b"12 5"].concat());
        cases.push([b"12".as_slice(), &[term], b"5"].concat());
    }
    // UTF-8 whitespace is NOT isspace() in the C locale.
    cases.push("12\u{a0}5".as_bytes().to_vec());
    cases.push("\u{2007}12 5".as_bytes().to_vec());
    for c in cases.iter() {
        ctx.cmp_both(c)?;
        n += 1;
    }
    Ok(n)
}

fn r18_line_endings(ctx: &Ctx) -> Result<usize, String> {
    let shapes = [
        "5\r\n3", "5\r\n3\r\n", "5\n3\n", "5\n3", "5\r3", "5\r3\r", "\r\n5\r\n3\r\n",
        "5\n\n\n3", "5\r\r\r3", "\n\n5 3\n\n", "5 3\r\n", "5 3\n",
    ];
    let mut n = 0;
    for s in shapes.iter() {
        ctx.cmp_both(s.as_bytes())?;
        n += 1;
    }
    Ok(n)
}

fn r19_arbitrary_bytes(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 19);
    let mut n = 0;
    for _ in 0..(ctx.cases * 3) {
        let len = rng.below(25);
        let bytes: Vec<u8> = (0..len).map(|_| (rng.next_u64() & 0xff) as u8).collect();
        ctx.cmp_so(&bytes, &Env::default())?;
        n += 1;
    }
    for _ in 0..ctx.cases {
        let len = rng.below(25);
        let bytes: Vec<u8> = (0..len).map(|_| (rng.next_u64() & 0xff) as u8).collect();
        ctx.cmp_exe(&bytes, &Env::default())?;
        n += 1;
    }
    Ok(n)
}

fn r20_hostile_alphabet_fuzz(ctx: &Ctx) -> Result<usize, String> {
    const ALPHA: &[u8] = b"0123456789+- \t\n\x0b\x0c\rxXeE.,/\0\xffaZ";
    let mut rng = Rng::new(SEED ^ 20);
    let mut n = 0;
    for _ in 0..(ctx.cases * 3) {
        let len = rng.below(22);
        let bytes: Vec<u8> = (0..len).map(|_| ALPHA[rng.below(ALPHA.len())]).collect();
        ctx.cmp_so(&bytes, &Env::default())?;
        n += 1;
    }
    for _ in 0..ctx.cases {
        let len = rng.below(22);
        let bytes: Vec<u8> = (0..len).map(|_| ALPHA[rng.below(ALPHA.len())]).collect();
        ctx.cmp_exe(&bytes, &Env::default())?;
        n += 1;
    }
    Ok(n)
}

fn r21_stdin_kinds(ctx: &Ctx) -> Result<usize, String> {
    let inputs: [&[u8]; 6] = [b"9 4", b"", b"7", b"q", b"-9 -4", b"  12\t-5  "];
    let mut n = 0;
    for kind in [
        StdinKind::File,
        StdinKind::Pipe,
        StdinKind::DevNull,
        StdinKind::Closed,
    ] {
        for inp in inputs.iter() {
            let env = Env {
                stdin: kind,
                ..Env::default()
            };
            ctx.cmp_exe(inp, &env)?;
            ctx.cmp_so(inp, &env)?;
            n += 2;
        }
    }
    Ok(n)
}

fn r22_stdout_kinds(ctx: &Ctx) -> Result<usize, String> {
    let inputs: [&[u8]; 4] = [b"9 4", b"", b"-2147483648 3", b"5 0"];
    let mut n = 0;
    for kind in [
        StdoutKind::Pipe,
        StdoutKind::RegularFile,
        StdoutKind::DevNull,
        StdoutKind::DevFull,
        StdoutKind::Closed,
        StdoutKind::BrokenPipe,
    ] {
        for inp in inputs.iter() {
            let env = Env {
                stdout: kind,
                ..Env::default()
            };
            ctx.cmp_exe(inp, &env)?;
            n += 1;
        }
    }
    Ok(n)
}

fn r23_sigpipe_disposition(ctx: &Ctx) -> Result<usize, String> {
    let inputs: [&[u8]; 3] = [b"9 4", b"", b"5 0"];
    let mut n = 0;
    for disp in [SIG_DFL, SIG_IGN] {
        for out in [StdoutKind::Pipe, StdoutKind::BrokenPipe, StdoutKind::DevFull] {
            for inp in inputs.iter() {
                let env = Env {
                    stdout: out,
                    sigpipe: disp,
                    ..Env::default()
                };
                ctx.cmp_exe(inp, &env)?;
                n += 1;
            }
        }
    }
    Ok(n)
}

fn r24_sigfpe_disposition(ctx: &Ctx) -> Result<usize, String> {
    let inputs: [&[u8]; 5] = [
        b"5 0",
        b"0 0",
        b"-2147483648 -1",
        b"7 2",
        b"-2147483648 1",
    ];
    let mut n = 0;
    for (ign, blk) in [(false, false), (true, false), (false, true), (true, true)] {
        for inp in inputs.iter() {
            let env = Env {
                sigfpe_ignore: ign,
                sigfpe_block: blk,
                ..Env::default()
            };
            ctx.cmp_exe(inp, &env)?;
            ctx.cmp_so(inp, &env)?;
            n += 2;
        }
    }
    Ok(n)
}

fn r25_return_value_is_zero(ctx: &Ctx) -> Result<usize, String> {
    // `int main(){ ... return 0; }` -- every non-fatal path must yield exactly
    // exit code 0 with no signal, on BOTH sides and through BOTH entry points.
    let inputs: [&[u8]; 8] = [
        b"9 4",
        b"",
        b"q",
        b"7",
        b"-2147483648 3",
        b"9999999999999999999999 7",
        b"0x10",
        b"  \t\n ",
    ];
    let env = Env::default();
    let mut n = 0;
    for inp in inputs.iter() {
        for (label, got) in [
            ("C exe", ctx.run_c_exe(inp, &env)),
            ("Rust exe", ctx.run_rust_exe(inp, &env)),
            ("C so", ctx.run_c_so(inp, &env)),
            ("Rust so", ctx.run_rust_so(inp, &env)),
        ] {
            if got.code != Some(0) || got.signal.is_some() {
                return Err(format!(
                    "{label} did not `return 0` for input={}: {}",
                    pretty(inp),
                    got.render()
                ));
            }
            n += 1;
        }
    }
    Ok(n)
}

fn r27_extra_argv(ctx: &Ctx) -> Result<usize, String> {
    // `int main()` takes no parameters, so argv beyond argv[0] is invisible.
    const SETS: [&[&str]; 5] = [
        &[],
        &["1"],
        &["--help"],
        &["a", "b", "c", "d", "e"],
        &["-2147483648", "0", "\u{ff}\u{fe}"],
    ];
    let inputs: [&[u8]; 4] = [b"9 4", b"", b"q", b"5 0"];
    let mut n = 0;
    for extra in SETS {
        let env = Env {
            argv_extra: extra,
            ..Env::default()
        };
        for inp in inputs.iter() {
            ctx.cmp_exe(inp, &env)?;
            n += 1;
        }
    }
    // argv must not change the answer either.
    let base = ctx.run_c_exe(b"9 4", &Env::default());
    for extra in SETS {
        let env = Env {
            argv_extra: extra,
            ..Env::default()
        };
        for (label, got) in [
            ("C", ctx.run_c_exe(b"9 4", &env)),
            ("Rust", ctx.run_rust_exe(b"9 4", &env)),
        ] {
            if got != base {
                return Err(format!(
                    "{label} changed behaviour for argv {extra:?}: {} vs baseline {}",
                    got.render(),
                    base.render()
                ));
            }
        }
        n += 1;
    }
    Ok(n)
}

fn r28_stdin_residual_seekable(ctx: &Ctx) -> Result<usize, String> {
    // How much of a *seekable* stdin the program leaves for the next reader.
    // glibc reads a whole st_blksize block and then rewinds the descriptor from
    // _IO_cleanup(); this row pins that side effect down, including exactly at
    // the block boundary where ungetc() could otherwise land in a backup area.
    let mut rng = Rng::new(SEED ^ 28);
    let env = Env::default();
    let mut n = 0;

    let mut cases: Vec<Vec<u8>> = vec![
        b"7 3 tail".to_vec(),
        b"".to_vec(),
        b"7".to_vec(),
        b"q".to_vec(),
        b"3 2\nmore\nlines\n".to_vec(),
        b"  \t 12 \t -5 \t rest".to_vec(),
        b"0x10 rest".to_vec(),
        b"9 4".to_vec(),
        b"9 4\n".to_vec(),
        b"-".to_vec(),
        b"5 0 tail".to_vec(),                  // dies before _IO_cleanup runs
        b"-2147483648 -1 tail".to_vec(),       // ditto
        b"0 0 tail".to_vec(),                  // ditto
    ];
    // ungetc()-heavy shapes: a matching failure pushes the offending byte back,
    // so it must count as UNconsumed when _IO_new_file_sync computes the rewind.
    // An off-by-one here is exactly what these anchors pin down.
    for c in [
        &b"- 5 tail"[..],
        b"+ 5 tail",
        b"-q tail",
        b"--5 tail",
        b"7 - tail",
        b"7 -q tail",
        b"q 7 tail",
        b"0x10 tail",
        b"5.5 2 tail",
        b"12,34 tail",
        b"7q 3 tail",
        b"\xff 3 tail",
        b"\x005 2 tail",
        b"5 -0 tail",
        b"010 3 tail",
        b"3 4e2 tail",
        b"1e 2 tail",
        b"  \t\n",
        b"7 -",
        b"5",
        b"+",
    ] {
        cases.push(c.to_vec());
    }
    // Block-boundary probes: st_blksize is 4096 here, so make the byte that gets
    // ungetc()'d land on either side of the buffer edge.
    for pad in [4090usize, 4093, 4094, 4095, 4096, 4097, 4098, 8190, 8191, 8192, 8193] {
        cases.push([b"1".repeat(pad).as_slice(), b" 2 tail"].concat());
        cases.push([b"1".repeat(pad).as_slice(), b"\n2 tail"].concat());
    }
    // Inputs spanning several blocks.
    for len in [4096usize, 8192, 10_000, 20_000] {
        cases.push([b"9 4 ".as_slice(), &b"Z".repeat(len)].concat());
        cases.push([digits(&mut rng, len).as_bytes(), b" 7 tail"].concat());
    }
    for _ in 0..ctx.cases {
        let len = rng.below(60);
        cases.push((0..len).map(|_| (rng.next_u64() & 0xff) as u8).collect());
    }

    for c in cases.iter() {
        let quad = ctx.all_four(c, &env);
        quad.verify(c)?;
        if quad.c_exe.stdin_residual.is_none() {
            return Err("stdin residual was not measured".to_string());
        }
        n += 1;
    }
    Ok(n)
}

fn r29_stdin_residual_pipe(ctx: &Ctx) -> Result<usize, String> {
    // Same thing for an *unseekable* stdin: the ESPIPE from _IO_new_file_sync is
    // swallowed, so whatever the block read swallowed stays swallowed.
    let mut rng = Rng::new(SEED ^ 29);
    let env = Env {
        stdin: StdinKind::PipeResidual,
        ..Env::default()
    };
    let mut cases: Vec<Vec<u8>> = vec![
        b"7 3 tail".to_vec(),
        b"".to_vec(),
        b"7".to_vec(),
        b"q".to_vec(),
        b"3 2\nmore\nlines\n".to_vec(),
        b"9 4".to_vec(),
        b"5 0 tail".to_vec(),
    ];
    for len in [1000usize, 4090, 4095, 4096, 4097, 8192, 16_384, 40_000] {
        cases.push([b"9 4 ".as_slice(), &b"Z".repeat(len)].concat());
    }
    for _ in 0..ctx.cases {
        let len = rng.below(60);
        cases.push((0..len).map(|_| (rng.next_u64() & 0xff) as u8).collect());
    }
    let mut n = 0;
    for c in cases.iter() {
        let quad = ctx.all_four(c, &env);
        quad.verify(c)?;
        if quad.c_exe.stdin_residual.is_none() {
            return Err("pipe residual was not measured".to_string());
        }
        n += 1;
    }
    Ok(n)
}

fn r26_dlopen_resolve_close(ctx: &Ctx) -> Result<usize, String> {
    // Resolve `driver_main` by name in-process with libloading, for both
    // libraries, then dlclose.  Not called here (it would consume the test
    // process's stdin); the *calling* path is what every other row does.
    let mut n = 0;
    for so in [&ctx.c_so, &ctx.rust_so] {
        unsafe {
            let lib = libloading::Library::new(so)
                .map_err(|e| format!("dlopen({}) failed: {e}", so.display()))?;
            let sym: Result<libloading::Symbol<unsafe extern "C" fn() -> c_int>, _> =
                lib.get(b"driver_main\0");
            sym.map_err(|e| format!("driver_main missing from {}: {e}", so.display()))?;
            // A name the C library definitely does not export.
            let bogus: Result<libloading::Symbol<unsafe extern "C" fn() -> c_int>, _> =
                lib.get(b"driver_main_not_a_symbol\0");
            if bogus.is_ok() {
                return Err(format!(
                    "{} unexpectedly exports driver_main_not_a_symbol",
                    so.display()
                ));
            }
            drop(lib); // dlclose
        }
        n += 1;
    }
    Ok(n)
}

// ===========================================================================
// Phase C -- ERRORS.md rows
//
// Each row asserts the SAME rejection, not merely "both failed": the full
// Outcome (stdout bytes, stderr bytes, exit code, terminating signal,
// core-dump flag) must be equal, AND the row additionally asserts the concrete
// sentinel the C is documented to produce (the surviving `1` initialiser, or
// signal 8 / signal 13).
// ===========================================================================

/// Assert both sides agree AND that the C side produced exactly `expect_stdout`
/// with exit code 0 and no signal.
fn expect_quiet(ctx: &Ctx, input: &[u8], expect_stdout: &str) -> Result<(), String> {
    let quad = ctx.all_four(input, &Env::default());
    quad.verify(input)?;
    let c = &quad.c_exe;
    if c.stdout != expect_stdout.as_bytes() || c.code != Some(0) || c.signal.is_some() {
        return Err(format!(
            "C reference itself deviated for input={}: expected stdout {:?} exit 0, got {}",
            pretty(input),
            expect_stdout,
            c.render()
        ));
    }
    Ok(())
}

/// Assert both sides agree AND that both were killed by `sig` with no output.
fn expect_fatal(ctx: &Ctx, input: &[u8], env: &Env, sig: i32) -> Result<(), String> {
    let c = ctx.run_c_exe(input, env);
    let r = ctx.run_rust_exe(input, env);
    if c != r {
        return Err(format!(
            "[exe env={:?}] input={}\n      C:    {}\n      Rust: {}",
            env,
            pretty(input),
            c.render(),
            r.render()
        ));
    }
    if c.signal != Some(sig) {
        return Err(format!(
            "expected both to die from signal {sig} for input={}, C gave {}",
            pretty(input),
            c.render()
        ));
    }
    if !c.stdout.is_empty() {
        return Err(format!(
            "expected no stdout before signal {sig} for input={}, C gave {}",
            pretty(input),
            c.render()
        ));
    }
    Ok(())
}

/// Like `expect_fatal`, but also drives the `.so`/`driver_main` entry point and
/// checks all four artifacts (4 spawns).  Only valid for faults that are
/// reproduced identically inside the dlopen runner -- i.e. SIGFPE, which the
/// kernel force-delivers regardless of the runner's own signal state.
fn expect_fatal_both(ctx: &Ctx, input: &[u8], env: &Env, sig: i32) -> Result<(), String> {
    let quad = ctx.all_four(input, env);
    quad.verify(input)?;
    for (label, got) in [
        ("C exe", &quad.c_exe),
        ("Rust exe", &quad.rust_exe),
        ("C .so", &quad.c_so),
        ("Rust .so", &quad.rust_so),
    ] {
        if got.signal != Some(sig) {
            return Err(format!(
                "{label} should die from signal {sig} for input={} (env={env:?}), got {}",
                pretty(input),
                got.render()
            ));
        }
        if !got.stdout.is_empty() {
            return Err(format!(
                "{label} should print nothing before signal {sig} for input={}, got {}",
                pretty(input),
                got.render()
            ));
        }
    }
    Ok(())
}

const KEEP_BOTH: &str = "quotient: 1, remainder: 0\n"; // x == y == 1
const KEEP_Y: &str = "quotient: 7, remainder: 0\n"; // x == 7, y == 1

fn err_e1_eof_immediately(ctx: &Ctx) -> Result<usize, String> {
    expect_quiet(ctx, b"", KEEP_BOTH)?;
    Ok(1)
}

fn err_e2_whitespace_only(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 102);
    let mut n = 0;
    for s in [" ", "\t", "\n", "\u{b}", "\u{c}", "\r", "  \t\n\u{b}\u{c}\r ", "\n\n\n"] {
        expect_quiet(ctx, s.as_bytes(), KEEP_BOTH)?;
        n += 1;
    }
    for _ in 0..24 {
        let s = ws_run(&mut rng, 1, 12);
        expect_quiet(ctx, s.as_bytes(), KEEP_BOTH)?;
        n += 1;
    }
    Ok(n)
}

fn err_e3_matching_failure_first(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for s in [
        "q 7", ".", "/", "x", ",", "a", "Z", "e5 2", "*", "#", "(nil)", "\u{7f}", "q", ": 3",
        "  \n q 7", "\t.5 2", "]3 4",
    ] {
        expect_quiet(ctx, s.as_bytes(), KEEP_BOTH)?;
        n += 1;
    }
    Ok(n)
}

fn err_e4_sign_then_eof(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for s in ["-", "+", "  -", "\n+", "\t\t-"] {
        expect_quiet(ctx, s.as_bytes(), KEEP_BOTH)?;
        n += 1;
    }
    Ok(n)
}

fn err_e5_sign_then_nondigit(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for s in [
        "- 5", "+ 5", "-q", "+q", "--5 3", "++5 3", "-+5 3", "+-5 3", "-.5 2", "-\n5 3", "- ", "+.",
        "-x10 3",
    ] {
        expect_quiet(ctx, s.as_bytes(), KEEP_BOTH)?;
        n += 1;
    }
    Ok(n)
}

fn err_e6_nul_byte_first(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for c in [
        b"\x005 2".to_vec(),
        b"\x00".to_vec(),
        b"\x00\x00".to_vec(),
        b" \x00 5 2".to_vec(),
        b"\x00-5 2".to_vec(),
    ] {
        expect_quiet(ctx, &c, KEEP_BOTH)?;
        n += 1;
    }
    Ok(n)
}

fn err_e7_high_byte_first(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for b in [0x80u8, 0x9f, 0xa0, 0xc3, 0xe2, 0xff] {
        expect_quiet(ctx, &[b], KEEP_BOTH)?;
        expect_quiet(ctx, &[b, b'5', b' ', b'2'], KEEP_BOTH)?;
        expect_quiet(ctx, &[b' ', b, b'5'], KEEP_BOTH)?;
        n += 3;
    }
    Ok(n)
}

fn err_e8_eof_after_x(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for s in ["7", "7 ", "7\n", "7\t", "7 \n\t\u{b}\u{c}\r ", "  7  "] {
        expect_quiet(ctx, s.as_bytes(), KEEP_Y)?;
        n += 1;
    }
    Ok(n)
}

fn err_e9_matching_failure_second(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for s in [
        "7 q", "7 .", "7,2", "7 /", "7 x", "7 a", "7\nq", "7 \t]", "7q", "7 e5", "7 (nil)",
    ] {
        expect_quiet(ctx, s.as_bytes(), KEEP_Y)?;
        n += 1;
    }
    Ok(n)
}

fn err_e10_second_sign_then_eof(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for s in ["7 -", "7 +", "7-", "7+", "7\n-"] {
        expect_quiet(ctx, s.as_bytes(), KEEP_Y)?;
        n += 1;
    }
    Ok(n)
}

fn err_e11_second_sign_then_nondigit(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for s in ["7 -q", "7 - 2", "7 --2", "7 +.", "7 -\n2", "7 ++2"] {
        expect_quiet(ctx, s.as_bytes(), KEEP_Y)?;
        n += 1;
    }
    Ok(n)
}

fn err_e12_positive_long_clamp(ctx: &Ctx) -> Result<usize, String> {
    // (int)LONG_MAX == -1, so "<huge> 1" prints quotient -1.
    expect_quiet(ctx, b"9223372036854775808 1", "quotient: -1, remainder: 0\n")?;
    let mut rng = Rng::new(SEED ^ 112);
    let mut n = 1;
    for len in [19usize, 20, 25, 64, 512, 5000] {
        // a leading 9 guarantees the magnitude exceeds LONG_MAX for len >= 19
        let d = format!("9{}", digits(&mut rng, len - 1));
        expect_quiet(ctx, format!("{d} 1").as_bytes(), "quotient: -1, remainder: 0\n")?;
        ctx.cmp_both(format!("{d} 7").as_bytes())?;
        n += 2;
    }
    for s in [
        "9223372036854775808 1",
        "9999999999999999999 1",
        "18446744073709551616 1",
        "99999999999999999999999999 1",
    ] {
        expect_quiet(ctx, s.as_bytes(), "quotient: -1, remainder: 0\n")?;
        n += 1;
    }
    Ok(n)
}

fn err_e13_negative_long_clamp(ctx: &Ctx) -> Result<usize, String> {
    // (int)LONG_MIN == 0, so "-<huge> 1" prints quotient 0 remainder 0.
    let zero = "quotient: 0, remainder: 0\n";
    let mut n = 0;
    for s in [
        "-9223372036854775809 1",
        "-9999999999999999999 1",
        "-18446744073709551616 1",
        "-99999999999999999999999999 1",
    ] {
        expect_quiet(ctx, s.as_bytes(), zero)?;
        n += 1;
    }
    let mut rng = Rng::new(SEED ^ 113);
    for len in [19usize, 20, 25, 64, 512, 5000] {
        let d = format!("9{}", digits(&mut rng, len - 1));
        expect_quiet(ctx, format!("-{d} 1").as_bytes(), zero)?;
        ctx.cmp_both(format!("-{d} 7").as_bytes())?;
        n += 2;
    }
    Ok(n)
}

fn err_e14_long_to_int_truncation(ctx: &Ctx) -> Result<usize, String> {
    let expected: [(&str, &str); 6] = [
        ("4294967296 1", "quotient: 0, remainder: 0\n"),
        ("4294967297 1", "quotient: 1, remainder: 0\n"),
        ("2147483648 1", "quotient: -2147483648, remainder: 0\n"),
        ("-2147483649 1", "quotient: 2147483647, remainder: 0\n"),
        ("9223372036854775807 1", "quotient: -1, remainder: 0\n"),
        ("-9223372036854775808 1", "quotient: 0, remainder: 0\n"),
    ];
    let mut n = 0;
    for (inp, out) in expected.iter() {
        expect_quiet(ctx, inp.as_bytes(), out)?;
        n += 1;
    }
    // The truncated y can even become 0 and therefore trap.
    expect_fatal(ctx, b"5 4294967296", &Env::default(), 8)?;
    expect_fatal(ctx, b"5 -4294967296", &Env::default(), 8)?;
    n += 2;
    Ok(n)
}

fn err_e15_hex_prefix_quirk(ctx: &Ctx) -> Result<usize, String> {
    let zero = "quotient: 0, remainder: 0\n";
    let mut n = 0;
    for s in ["0x10", "0X10", "0x", "0X", "0xg", "0x 5"] {
        expect_quiet(ctx, s.as_bytes(), zero)?;
        n += 1;
    }
    // In the second position the same quirk leaves y == 0 -> trap.
    expect_fatal(ctx, b"7 0x10", &Env::default(), 8)?;
    n += 1;
    // "010" is decimal ten for %d, not octal eight.
    expect_quiet(ctx, b"010 3", "quotient: 3, remainder: 1\n")?;
    n += 1;
    Ok(n)
}

fn err_e16_division_by_zero(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 116);
    let env = Env::default();
    let mut n = 0;
    for s in ["5 0", "0 0", "-3 0", "5 -0", "5 +0", "-2147483648 0", "2147483647 0", "0 -0"] {
        expect_fatal_both(ctx, s.as_bytes(), &env, 8)?;
        n += 1;
    }
    for _ in 0..24 {
        let x = rng.range(i32::MIN as i128, i32::MAX as i128);
        let zero = *rng.pick(&["0", "-0", "+0", "00", "0000000000"]);
        let s = format!("{x} {zero}");
        expect_fatal_both(ctx, s.as_bytes(), &env, 8)?;
        n += 1;
    }
    Ok(n)
}

fn err_e17_int_min_over_minus_one(ctx: &Ctx) -> Result<usize, String> {
    let env = Env::default();
    let mut n = 0;
    for s in [
        "-2147483648 -1",
        "  -2147483648\t-1  ",
        "-2147483648 -0000001",
        "-00002147483648 -1",
        // via long->int truncation: 2147483648 truncates to INT_MIN
        "2147483648 -1",
        // via strtol clamp: -9223372036854775808 truncates to 0, not INT_MIN,
        // so this one must NOT trap -- guards against over-eager trapping.
    ] {
        expect_fatal_both(ctx, s.as_bytes(), &env, 8)?;
        n += 1;
    }
    expect_quiet(ctx, b"-9223372036854775808 -1", "quotient: 0, remainder: 0\n")?;
    // INT_MIN / +1 and INT_MIN / -2 must NOT trap.
    expect_quiet(ctx, b"-2147483648 1", "quotient: -2147483648, remainder: 0\n")?;
    expect_quiet(ctx, b"-2147483648 -2", "quotient: 1073741824, remainder: 0\n")?;
    // INT_MIN+1 / -1 is representable and must NOT trap.
    expect_quiet(ctx, b"-2147483647 -1", "quotient: 2147483647, remainder: 0\n")?;
    n += 4;
    Ok(n)
}

fn err_e18_sigfpe_ignored_or_blocked(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for (ign, blk) in [(true, false), (false, true), (true, true)] {
        let env = Env {
            sigfpe_ignore: ign,
            sigfpe_block: blk,
            ..Env::default()
        };
        for s in ["5 0", "0 0", "-2147483648 -1"] {
            // The kernel force-delivers synchronous faults, so SIG_IGN and a
            // blocked mask do not save the process.
            expect_fatal_both(ctx, s.as_bytes(), &env, 8)?;
            n += 1;
        }
    }
    Ok(n)
}

fn err_e19_epipe_sigpipe_default(ctx: &Ctx) -> Result<usize, String> {
    let env = Env {
        stdout: StdoutKind::BrokenPipe,
        sigpipe: SIG_DFL,
        ..Env::default()
    };
    let mut n = 0;
    for s in ["9 4", "", "q", "-2147483648 3"] {
        expect_fatal(ctx, s.as_bytes(), &env, 13)?;
        n += 1;
    }
    // ... but a trapping division still dies from SIGFPE first, before any write.
    expect_fatal(ctx, b"5 0", &env, 8)?;
    n += 1;
    Ok(n)
}

fn err_e20_epipe_sigpipe_ignored(ctx: &Ctx) -> Result<usize, String> {
    let env = Env {
        stdout: StdoutKind::BrokenPipe,
        sigpipe: SIG_IGN,
        ..Env::default()
    };
    let mut n = 0;
    for s in ["9 4", "", "q", "-2147483648 3"] {
        ctx.cmp_exe(s.as_bytes(), &env)?;
        let c = ctx.run_c_exe(s.as_bytes(), &env);
        if c.code != Some(0) || c.signal.is_some() || !c.stderr.is_empty() {
            return Err(format!(
                "C should exit 0 silently on EPIPE with SIGPIPE ignored, input={}: {}",
                pretty(s.as_bytes()),
                c.render()
            ));
        }
        let r = ctx.run_rust_exe(s.as_bytes(), &env);
        if r.code != Some(0) || r.signal.is_some() || !r.stderr.is_empty() {
            return Err(format!(
                "Rust must not panic/abort on EPIPE, input={}: {}",
                pretty(s.as_bytes()),
                r.render()
            ));
        }
        n += 1;
    }
    Ok(n)
}

fn err_e21_enospc_dev_full(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for sigpipe in [SIG_DFL, SIG_IGN] {
        let env = Env {
            stdout: StdoutKind::DevFull,
            sigpipe,
            ..Env::default()
        };
        for s in ["9 4", "", "q", "-2147483648 3"] {
            ctx.cmp_exe(s.as_bytes(), &env)?;
            for (label, got) in [
                ("C", ctx.run_c_exe(s.as_bytes(), &env)),
                ("Rust", ctx.run_rust_exe(s.as_bytes(), &env)),
            ] {
                if got.code != Some(0) || got.signal.is_some() || !got.stderr.is_empty() {
                    return Err(format!(
                        "{label} should exit 0 silently on ENOSPC, input={}: {}",
                        pretty(s.as_bytes()),
                        got.render()
                    ));
                }
            }
            n += 1;
        }
    }
    Ok(n)
}

fn err_e22_ebadf_stdout_closed(ctx: &Ctx) -> Result<usize, String> {
    let env = Env {
        stdout: StdoutKind::Closed,
        ..Env::default()
    };
    let mut n = 0;
    for s in ["9 4", "", "q"] {
        ctx.cmp_exe(s.as_bytes(), &env)?;
        for (label, got) in [
            ("C", ctx.run_c_exe(s.as_bytes(), &env)),
            ("Rust", ctx.run_rust_exe(s.as_bytes(), &env)),
        ] {
            if got.code != Some(0) || got.signal.is_some() || !got.stderr.is_empty() {
                return Err(format!(
                    "{label} should exit 0 silently with fd 1 closed, input={}: {}",
                    pretty(s.as_bytes()),
                    got.render()
                ));
            }
        }
        n += 1;
    }
    // fd 1 closed AND a trapping divide -> still SIGFPE.
    expect_fatal(ctx, b"5 0", &env, 8)?;
    n += 1;
    Ok(n)
}

fn err_e23_ebadf_stdin_closed(ctx: &Ctx) -> Result<usize, String> {
    let env = Env {
        stdin: StdinKind::Closed,
        ..Env::default()
    };
    let mut n = 0;
    for s in ["9 4", "", "q"] {
        ctx.cmp_exe(s.as_bytes(), &env)?;
        ctx.cmp_so(s.as_bytes(), &env)?;
        let c = ctx.run_c_exe(s.as_bytes(), &env);
        if c.stdout != KEEP_BOTH.as_bytes() || c.code != Some(0) {
            return Err(format!(
                "with fd 0 closed C must fall back to x=y=1, input={}: {}",
                pretty(s.as_bytes()),
                c.render()
            ));
        }
        n += 1;
    }
    Ok(n)
}

fn err_e24_zero_length_input(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for kind in [StdinKind::File, StdinKind::Pipe, StdinKind::DevNull] {
        let env = Env {
            stdin: kind,
            ..Env::default()
        };
        ctx.cmp_exe(b"", &env)?;
        ctx.cmp_so(b"", &env)?;
        let c = ctx.run_c_exe(b"", &env);
        if c.stdout != KEEP_BOTH.as_bytes() || c.code != Some(0) {
            return Err(format!("zero-length stdin ({kind:?}): C gave {}", c.render()));
        }
        n += 1;
    }
    Ok(n)
}

fn err_e25_oversized_input(ctx: &Ctx) -> Result<usize, String> {
    let mut rng = Rng::new(SEED ^ 125);
    let mut n = 0;
    // 1 MiB of digits with no separator: glibc must grow its scratch buffer and
    // then clamp in strtol.
    let big = format!("9{} 7", digits(&mut rng, 1024 * 1024 - 1));
    ctx.cmp_both(big.as_bytes())?;
    let c = ctx.run_c_exe(big.as_bytes(), &Env::default());
    if c.stdout != b"quotient: 0, remainder: -1\n" || c.code != Some(0) {
        return Err(format!("1 MiB digit run: C gave {}", c.render()));
    }
    n += 1;
    // 1 MiB of leading zeros followed by a small number: in range, no clamp.
    let zeros = format!("{}5 2", "0".repeat(1024 * 1024));
    ctx.cmp_both(zeros.as_bytes())?;
    n += 1;
    // 1 MiB of whitespace then a valid pair.
    let ws = format!("{}11 4", " ".repeat(1024 * 1024));
    ctx.cmp_both(ws.as_bytes())?;
    n += 1;
    // 1 MiB of junk -> matching failure.
    let junk = "q".repeat(1024 * 1024);
    ctx.cmp_both(junk.as_bytes())?;
    n += 1;
    Ok(n)
}

fn err_e26_one_past_int_range(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for v in [
        "2147483648",
        "-2147483649",
        "2147483647",
        "-2147483648",
        "2147483646",
        "-2147483647",
    ] {
        ctx.cmp_both(format!("{v} 3").as_bytes())?;
        ctx.cmp_both(format!("3 {v}").as_bytes())?;
        ctx.cmp_both(format!("{v} {v}").as_bytes())?;
        n += 3;
    }
    Ok(n)
}

fn err_e27_one_past_long_range(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for v in [
        "9223372036854775806",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775807",
        "-9223372036854775808",
        "-9223372036854775809",
    ] {
        ctx.cmp_both(format!("{v} 3").as_bytes())?;
        ctx.cmp_both(format!("3 {v}").as_bytes())?;
        n += 2;
    }
    Ok(n)
}

fn err_e29_all_single_bytes(ctx: &Ctx) -> Result<usize, String> {
    // `driver_main` takes no arguments, so the only bit patterns an external
    // caller can inject are stdin bytes.  Exhaust all 256 single-byte inputs...
    let mut n = 0;
    for b in 0u8..=255 {
        ctx.cmp_so(&[b], &Env::default())?;
        n += 1;
    }
    // ...and all 2-byte inputs over a hostile alphabet.
    const ALPHA: &[u8] = b"0123456789+- \t\n\r.,xX\0\xff";
    for &a in ALPHA {
        for &b in ALPHA {
            ctx.cmp_so(&[a, b], &Env::default())?;
            n += 1;
        }
    }
    // Return value of the exported symbol must be 0 whenever it returns at all.
    let r = ctx.run_rust_so(b"6 4", &Env::default());
    let c = ctx.run_c_so(b"6 4", &Env::default());
    if r.code != Some(0) || c.code != Some(0) {
        return Err(format!(
            "driver_main must return 0: C {} / Rust {}",
            c.render(),
            r.render()
        ));
    }
    n += 1;
    Ok(n)
}

/// The core-dump flag in `wait(2)`'s status word must match too.  Every other
/// row runs with RLIMIT_CORE = 0 (a core file costs ~0.4 s through
/// systemd-coredump), so this row -- and only this row -- lets cores through.
///
/// This is what distinguishes a *hardware* fault from a `raise(SIGFPE)`
/// look-alike: `raise` on a signal the caller has blocked or ignored would
/// silently return, whereas the kernel force-delivers a synchronous #DE.
fn err_e28_core_dump_flag(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for (ign, blk) in [(false, false), (true, false), (false, true)] {
        let env = Env {
            allow_core: true,
            sigfpe_ignore: ign,
            sigfpe_block: blk,
            ..Env::default()
        };
        for s in ["5 0", "-2147483648 -1"] {
            let quad = ctx.all_four(s.as_bytes(), &env);
            quad.verify(s.as_bytes())?;
            for (label, got) in [
                ("C exe", &quad.c_exe),
                ("Rust exe", &quad.rust_exe),
                ("C .so", &quad.c_so),
                ("Rust .so", &quad.rust_so),
            ] {
                if got.signal != Some(8) || !got.core_dumped {
                    return Err(format!(
                        "{label} should raise a core-dumping SIGFPE for input={s:?} (env={env:?}), got {}",
                        got.render()
                    ));
                }
            }
            n += 1;
        }
    }
    // A non-trapping divide must not dump core.
    let env = Env {
        allow_core: true,
        ..Env::default()
    };
    let quad = ctx.all_four(b"7 2", &env);
    quad.verify(b"7 2")?;
    if quad.c_exe.core_dumped || quad.rust_exe.core_dumped {
        return Err("a normal division must not dump core".to_string());
    }
    n += 1;
    Ok(n)
}

/// E30: `int main()` declares no parameters, so nothing an external caller puts
/// in the argument registers may matter.  This is the FFI-boundary analogue of
/// "pass an out-of-range enum value": `driver_main` is called through a
/// deliberately mis-declared six-integer-argument fn pointer whose values
/// include `u64::MAX`, `0x8000_0000_0000_0000` and other patterns that are not
/// valid values of anything.  Both libraries must be unmoved.
fn err_e30_garbage_argument_registers(ctx: &Ctx) -> Result<usize, String> {
    let mut n = 0;
    for inp in [
        &b"9 4"[..],
        b"",
        b"q",
        b"5 0",
        b"-2147483648 -1",
        b"9999999999999999999999 7",
        b"0x10",
    ] {
        let plain = Env::default();
        let garbage = Env {
            garbage_args: true,
            ..Env::default()
        };

        // C vs Rust, both called with garbage in the argument registers.
        let c = ctx.run_c_so(inp, &garbage);
        let r = ctx.run_rust_so(inp, &garbage);
        if c != r {
            return Err(format!(
                "[.so, garbage arg registers] input={}\n      C:    {}\n      Rust: {}",
                pretty(inp),
                c.render(),
                r.render()
            ));
        }
        // ... and identical to the zero-argument call, for both.
        for (label, garb, base) in [
            ("C", &c, ctx.run_c_so(inp, &plain)),
            ("Rust", &r, ctx.run_rust_so(inp, &plain)),
        ] {
            if *garb != base {
                return Err(format!(
                    "{label} .so reacted to garbage argument registers for input={}\n      with args: {}\n      without:   {}",
                    pretty(inp),
                    garb.render(),
                    base.render()
                ));
            }
        }
        n += 1;
    }
    Ok(n)
}

/// E31: `_IO_new_file_sync`'s exit-time `lseek` fails with `ESPIPE` on an
/// unseekable stdin and glibc swallows the error, so the whole `st_blksize`
/// block that was read stays consumed and the exit status stays 0.
fn err_e31_espipe_swallowed(ctx: &Ctx) -> Result<usize, String> {
    let env = Env {
        stdin: StdinKind::PipeResidual,
        ..Env::default()
    };
    let block = 4096u64; // st_blksize of a pipe on Linux
    let mut n = 0;
    for total in [8192usize, 12_000usize, 20_000usize] {
        let input = [b"9 4 ".as_slice(), &b"Z".repeat(total - 4)].concat();
        let quad = ctx.all_four(&input, &env);
        quad.verify(&input)?;
        for (label, got) in [
            ("C exe", &quad.c_exe),
            ("Rust exe", &quad.rust_exe),
            ("C .so", &quad.c_so),
            ("Rust .so", &quad.rust_so),
        ] {
            if got.code != Some(0) || got.signal.is_some() {
                return Err(format!(
                    "{label}: ESPIPE must be swallowed, got {}",
                    got.render()
                ));
            }
            let want = total as u64 - block;
            if got.stdin_residual != Some(want) {
                return Err(format!(
                    "{label}: expected {want} bytes left in the pipe (a whole \
                     {block}-byte block consumed and never given back), got {:?}",
                    got.stdin_residual
                ));
            }
        }
        n += 1;
    }
    Ok(n)
}

/// E32: when the process dies from a signal, `_IO_cleanup()` never runs, so the
/// exit-time rewind must NOT happen -- the descriptor is left wherever the block
/// read put it.
fn err_e32_no_rewind_when_signalled(ctx: &Ctx) -> Result<usize, String> {
    let env = Env::default(); // seekable file stdin
    let block = 4096u64;
    let mut n = 0;

    // (a) file shorter than one block: the whole file is swallowed.
    for inp in [&b"5 0 tail"[..], b"0 0 tail", b"-2147483648 -1 tail"] {
        let quad = ctx.all_four(inp, &env);
        quad.verify(inp)?;
        for (label, got) in [("C exe", &quad.c_exe), ("Rust exe", &quad.rust_exe)] {
            if got.signal != Some(8) {
                return Err(format!("{label}: expected SIGFPE, got {}", got.render()));
            }
            if got.stdin_residual != Some(inp.len() as u64) {
                return Err(format!(
                    "{label}: offset must stay at the end of the block read ({}), got {:?}",
                    inp.len(),
                    got.stdin_residual
                ));
            }
        }
        n += 1;
    }

    // (b) file longer than one block: exactly one block is swallowed.
    let long = [b"5 0 ".as_slice(), &b"Z".repeat(20_000)].concat();
    let quad = ctx.all_four(&long, &env);
    quad.verify(&long)?;
    for (label, got) in [("C exe", &quad.c_exe), ("Rust exe", &quad.rust_exe)] {
        if got.signal != Some(8) || got.stdin_residual != Some(block) {
            return Err(format!(
                "{label}: expected SIGFPE with the offset left at {block}, got {}",
                got.render()
            ));
        }
    }
    n += 1;

    // Control: the same input with a non-trapping divisor DOES get rewound.
    let ok = [b"5 2 ".as_slice(), &b"Z".repeat(20_000)].concat();
    let quad = ctx.all_four(&ok, &env);
    quad.verify(&ok)?;
    for (label, got) in [("C exe", &quad.c_exe), ("Rust exe", &quad.rust_exe)] {
        if got.signal.is_some() || got.stdin_residual != Some(3) {
            return Err(format!(
                "{label}: a clean exit must rewind to offset 3, got {}",
                got.render()
            ));
        }
    }
    n += 1;
    Ok(n)
}

// ===========================================================================
// Phase D -- symbol parity
// ===========================================================================

fn nm_defined(so: &Path) -> Result<Vec<String>, String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", so.to_str().unwrap()])
        .output()
        .map_err(|e| format!("nm: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "nm -D {} failed: {}",
            so.display(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| !s.starts_with("_ITM_") && !s.starts_with("__cxa_") && s != "__gmon_start__")
        .collect();
    v.sort();
    v.dedup();
    Ok(v)
}

fn symbol_parity(ctx: &Ctx) -> Result<usize, String> {
    let c = nm_defined(&ctx.c_so)?;
    let r = nm_defined(&ctx.rust_so)?;
    if c != ["driver_main"] {
        return Err(format!("unexpected C .so export set: {c:?}"));
    }
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    if !missing.is_empty() {
        return Err(format!(
            "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n      C:    {c:?}\n      Rust: {r:?}"
        ));
    }
    if c != r {
        return Err(format!(
            "export sets differ (extra Rust symbols): C={c:?} Rust={r:?}"
        ));
    }
    // No unresolvable non-libc imports on either side.
    for so in [&ctx.c_so, &ctx.rust_so] {
        let out = Command::new("ldd")
            .arg(so.to_str().unwrap())
            .output()
            .map_err(|e| format!("ldd: {e}"))?;
        let text = String::from_utf8_lossy(&out.stdout);
        if text.contains("not found") {
            return Err(format!("{} has unresolved imports:\n{text}", so.display()));
        }
    }
    Ok(c.len())
}

// ===========================================================================
// Driver
// ===========================================================================

type Row = (&'static str, &'static str, fn(&Ctx) -> Result<usize, String>);

fn rows() -> Vec<Row> {
    vec![
        // Phase D (cheap, run first so a partial translation fails fast)
        ("SYM", "nm -D export parity C .so vs Rust .so", symbol_parity),
        // Phase B -- CONFIGS.md
        ("R1", "canonical pair, full i32 range", r1_canonical_full_range),
        ("R2", "small operands, all sign quadrants", r2_small_all_quadrants),
        ("R3", "exact division (remainder 0)", r3_exact_division),
        ("R4", "|x| < |y|", r4_dividend_smaller),
        ("R5", "x == 0 / y == +-1 / x == y", r5_zero_and_unit_operands),
        ("R6", "int boundary cross product", r6_int_boundary_cross_product),
        ("R7", "whitespace matrix", r7_whitespace_matrix),
        ("R8", "explicit +/- signs, signed zero", r8_explicit_signs),
        ("R9", "leading zeros", r9_leading_zeros),
        ("R10", "long->int truncation range", r10_long_to_int_truncation),
        ("R11", "strtol LONG_MAX/LONG_MIN clamp", r11_long_range_clamping),
        ("R12", "100..5000-digit operands", r12_very_long_digit_strings),
        ("R13", "single token then EOF/junk", r13_single_token),
        ("R14", "more than two tokens", r14_more_than_two_tokens),
        ("R15", "glibc base-prefix shapes", r15_base_prefix_shapes),
        ("R16", "non-digit terminators", r16_nondigit_terminators),
        ("R17", "NUL / high-byte terminators", r17_nul_and_high_byte_terminators),
        ("R18", "CR / CRLF line endings", r18_line_endings),
        ("R19", "fuzz: arbitrary bytes 0..255", r19_arbitrary_bytes),
        ("R20", "fuzz: hostile alphabet", r20_hostile_alphabet_fuzz),
        ("R21", "stdin kind matrix", r21_stdin_kinds),
        ("R22", "stdout kind matrix", r22_stdout_kinds),
        ("R23", "inherited SIGPIPE disposition", r23_sigpipe_disposition),
        ("R24", "inherited SIGFPE disposition", r24_sigfpe_disposition),
        ("R25", "entry point returns 0", r25_return_value_is_zero),
        ("R26", "dlopen / resolve by name / dlclose", r26_dlopen_resolve_close),
        ("R27", "extra argv is ignored", r27_extra_argv),
        ("R28", "residual offset, seekable stdin", r28_stdin_residual_seekable),
        ("R29", "residual bytes, pipe stdin", r29_stdin_residual_pipe),
        // Phase C -- ERRORS.md
        ("E1", "scanf#1 input failure: EOF", err_e1_eof_immediately),
        ("E2", "scanf#1 input failure: ws only", err_e2_whitespace_only),
        ("E3", "scanf#1 matching failure", err_e3_matching_failure_first),
        ("E4", "scanf#1 sign then EOF", err_e4_sign_then_eof),
        ("E5", "scanf#1 sign then non-digit", err_e5_sign_then_nondigit),
        ("E6", "scanf#1 NUL byte first", err_e6_nul_byte_first),
        ("E7", "scanf#1 byte >= 0x80 first", err_e7_high_byte_first),
        ("E8", "scanf#2 input failure: EOF", err_e8_eof_after_x),
        ("E9", "scanf#2 matching failure", err_e9_matching_failure_second),
        ("E10", "scanf#2 sign then EOF", err_e10_second_sign_then_eof),
        ("E11", "scanf#2 sign then non-digit", err_e11_second_sign_then_nondigit),
        ("E12", "strtol positive clamp -> (int)-1", err_e12_positive_long_clamp),
        ("E13", "strtol negative clamp -> (int)0", err_e13_negative_long_clamp),
        ("E14", "long->int truncation", err_e14_long_to_int_truncation),
        ("E15", "0x prefix quirk", err_e15_hex_prefix_quirk),
        ("E16", "div by zero -> SIGFPE", err_e16_division_by_zero),
        ("E17", "INT_MIN / -1 -> SIGFPE", err_e17_int_min_over_minus_one),
        ("E18", "SIGFPE ignored/blocked still kills", err_e18_sigfpe_ignored_or_blocked),
        ("E19", "EPIPE, SIGPIPE default -> SIGPIPE", err_e19_epipe_sigpipe_default),
        ("E20", "EPIPE, SIGPIPE ignored -> exit 0", err_e20_epipe_sigpipe_ignored),
        ("E21", "ENOSPC (/dev/full) -> exit 0", err_e21_enospc_dev_full),
        ("E22", "EBADF (fd 1 closed) -> exit 0", err_e22_ebadf_stdout_closed),
        ("E23", "EBADF (fd 0 closed) -> x=y=1", err_e23_ebadf_stdin_closed),
        ("E24", "zero-length stdin", err_e24_zero_length_input),
        ("E25", "oversized (1 MiB) stdin", err_e25_oversized_input),
        ("E26", "one past int range", err_e26_one_past_int_range),
        ("E27", "one past long range", err_e27_one_past_long_range),
        ("E28", "core-dump flag parity (hw fault)", err_e28_core_dump_flag),
        ("E29", "all 256 single bytes + 2-byte grid", err_e29_all_single_bytes),
        ("E30", "garbage in argument registers", err_e30_garbage_argument_registers),
        ("E31", "ESPIPE on exit-time lseek swallowed", err_e31_espipe_swallowed),
        ("E32", "no rewind when killed by a signal", err_e32_no_rewind_when_signalled),
    ]
}

fn main() {
    // --- runner mode -------------------------------------------------------
    if let Ok(so) = std::env::var(RUNNER_ENV) {
        run_as_so_runner(PathBuf::from(so));
    }

    let ctx = std::sync::Arc::new(Ctx::discover());
    let only = std::env::var("DRIVER_DIFF_ONLY").ok();

    println!("C exe   : {}", ctx.c_exe.display());
    println!("Rust exe: {}", ctx.rust_exe.display());
    println!("C .so   : {}", ctx.c_so.display());
    println!("Rust .so: {}", ctx.rust_so.display());
    println!("randomized cases per row: {}", ctx.cases);

    let selected: Vec<Row> = rows()
        .into_iter()
        .filter(|(id, _, _)| match &only {
            Some(o) => o.split(',').any(|p| p.trim() == *id),
            None => true,
        })
        .collect();

    // Rows are independent; every case is a fresh process on both sides, so the
    // rows can be evaluated concurrently.  This matters because a case that
    // ends in SIGFPE costs ~0.4 s of systemd-coredump time.
    let threads: usize = std::env::var("DRIVER_DIFF_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8)
        .clamp(1, 64);
    println!("worker threads: {threads}\n");

    let next = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let selected = std::sync::Arc::new(selected);
    let results: std::sync::Arc<std::sync::Mutex<Vec<Option<(Result<usize, String>, f64)>>>> =
        std::sync::Arc::new(std::sync::Mutex::new(vec![None; selected.len()]));

    let start = std::time::Instant::now();
    let mut handles = Vec::new();
    for _ in 0..threads.min(selected.len().max(1)) {
        let ctx = ctx.clone();
        let next = next.clone();
        let selected = selected.clone();
        let results = results.clone();
        handles.push(std::thread::spawn(move || loop {
            let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if i >= selected.len() {
                break;
            }
            let (id, desc, f) = selected[i];
            let t = std::time::Instant::now();
            let outcome = f(&ctx);
            let secs = t.elapsed().as_secs_f64();
            match &outcome {
                Ok(n) => println!("  [PASS] {id:<4} {desc:<44} {n:>6} cases  {secs:>7.2}s"),
                Err(_) => println!("  [FAIL] {id:<4} {desc:<44}          {secs:>7.2}s"),
            }
            results.lock().unwrap()[i] = Some((outcome, secs));
        }));
    }
    for h in handles {
        h.join().expect("worker thread panicked");
    }

    let results = std::sync::Arc::try_unwrap(results)
        .expect("results still shared")
        .into_inner()
        .unwrap();

    let mut failures: Vec<(&str, String)> = Vec::new();
    let mut total_cases = 0usize;
    for (i, (id, _, _)) in selected.iter().enumerate() {
        match results[i].as_ref() {
            Some((Ok(n), _)) => total_cases += n,
            Some((Err(d), _)) => failures.push((id, d.clone())),
            None => failures.push((id, "row never ran".to_string())),
        }
    }

    println!(
        "\n{} rows run, {} cases, {:.2}s elapsed",
        selected.len(),
        total_cases,
        start.elapsed().as_secs_f64()
    );
    if failures.is_empty() {
        println!("RESULT: all {} rows PASS -- C and Rust are byte-identical", selected.len());
    } else {
        println!("RESULT: {} row(s) FAILED:", failures.len());
        for (id, d) in &failures {
            println!("  {id}: {d}");
        }
        std::process::exit(1);
    }
}
