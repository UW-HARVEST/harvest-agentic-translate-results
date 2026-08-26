//! Shared support code for the C-vs-Rust differential tests.
//!
//! Nothing in the Rust crate is ever called directly: both implementations are
//! reached exclusively through their built artifacts —
//!
//! * `libc_driver.so`   — `c_src/src/main.c` compiled with `gcc -shared -fPIC`
//! * `libdriver.so`     — the Rust `cdylib`
//! * `c_src/build/driver` / `target/<profile>/driver` — the two executables
//!
//! so the `#[no_mangle] extern "C"` wrappers are part of what is under test.

#![allow(dead_code)]

use std::io::Write;
use std::os::raw::{c_int, c_uint};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits needed to capture what the loaded libraries write to fd 1
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* libc output streams, i.e. the `stdout`
    /// FILE that the C shared object's `printf` writes into.
    fn fflush(stream: *mut u8) -> c_int;
}

// ---------------------------------------------------------------------------
// Paths / artifacts
// ---------------------------------------------------------------------------

pub fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` of the currently running test binary.
pub fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // …/target/<profile>/deps/<test>-<hash>
    exe.parent()
        .and_then(Path::parent)
        .expect("profile dir")
        .to_path_buf()
}

fn scratch_dir() -> PathBuf {
    let d = crate_root().join("target").join("cdiff");
    std::fs::create_dir_all(&d).expect("create target/cdiff");
    d
}

fn newer(a: &Path, b: &Path) -> bool {
    let ma = std::fs::metadata(a).and_then(|m| m.modified());
    let mb = std::fs::metadata(b).and_then(|m| m.modified());
    match (ma, mb) {
        (Ok(ta), Ok(tb)) => ta > tb,
        _ => true,
    }
}

fn c_source() -> PathBuf {
    crate_root().join("c_src").join("src").join("main.c")
}

/// The C translation unit as a shared object (built once per test run;
/// `c_src/` itself is never touched — the output goes to `target/cdiff/`).
pub fn c_shared_lib() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let out = scratch_dir().join("libc_driver.so");
        if !out.exists() || newer(&c_source(), &out) {
            let st = Command::new("gcc")
                .args(["-shared", "-fPIC", "-o"])
                .arg(&out)
                .arg(c_source())
                .status()
                .expect("run gcc");
            assert!(st.success(), "gcc -shared failed");
        }
        out
    })
    .clone()
}

/// The C executable.  Prefers the artifact produced by `c_src/CMakeLists.txt`
/// (`c_src/build/driver`); falls back to an equivalent `gcc` build.
pub fn c_exe() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let cmake_built = crate_root().join("c_src").join("build").join("driver");
        if cmake_built.exists() && !newer(&c_source(), &cmake_built) {
            return cmake_built;
        }
        let out = scratch_dir().join("c_driver");
        if !out.exists() || newer(&c_source(), &out) {
            let st = Command::new("gcc")
                .arg("-o")
                .arg(&out)
                .arg(c_source())
                .status()
                .expect("run gcc");
            assert!(st.success(), "gcc failed");
        }
        out
    })
    .clone()
}

/// The Rust `cdylib` (`[[example]] name = "cdylib"`, so `cargo test` builds it
/// with the same profile and features as the test binary itself).
pub fn rust_shared_lib() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let so = profile_dir().join("examples").join("libcdylib.so");
        if so.exists() {
            return so;
        }
        // Fallback: build it explicitly in a separate target directory (its own
        // lock, so it cannot deadlock against the outer cargo invocation).
        let release = profile_dir()
            .file_name()
            .map(|n| n == "release")
            .unwrap_or(false);
        let target = crate_root().join("target").join("ffi_so");
        let mut cmd = Command::new(env!("CARGO"));
        cmd.current_dir(crate_root())
            .env("CARGO_TARGET_DIR", &target)
            .args(["build", "--offline", "--example", "cdylib"]);
        if release {
            cmd.arg("--release");
        }
        if !cfg!(feature = "default") {
            cmd.arg("--no-default-features");
        }
        let out = cmd.output().expect("run cargo build --example cdylib");
        assert!(
            out.status.success(),
            "cargo build --example cdylib failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let so = target
            .join(if release { "release" } else { "debug" })
            .join("examples")
            .join("libcdylib.so");
        assert!(so.exists(), "{} not produced", so.display());
        so
    })
    .clone()
}

/// The Rust executable (built by `cargo test` because integration tests exist).
pub fn rust_exe() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let p = profile_dir().join("driver");
        assert!(p.exists(), "{} not built", p.display());
        p
    })
    .clone()
}

/// The `examples/so_runner.rs` helper (built by `cargo test`).
pub fn so_runner() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let p = profile_dir().join("examples").join("so_runner");
        assert!(p.exists(), "{} not built", p.display());
        p
    })
    .clone()
}

// ---------------------------------------------------------------------------
// In-process FFI access (libloading)
// ---------------------------------------------------------------------------

pub type DriverFn = unsafe extern "C" fn(c_uint, c_uint, u8, c_int);
pub type PrintFooFn = unsafe extern "C" fn(*const u8);

/// `foo_t` as raw bytes: 8 bytes, 4-byte aligned (verified against gcc).
#[repr(C, align(4))]
pub struct RawFoo(pub [u8; 8]);

impl RawFoo {
    pub fn new(bits: u8, pad: [u8; 3], z: i32) -> RawFoo {
        let mut r = RawFoo([0u8; 8]);
        r.0[0] = bits;
        r.0[1..4].copy_from_slice(&pad);
        r.0[4..8].copy_from_slice(&z.to_ne_bytes());
        r
    }
    pub fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }
}

pub struct Impl {
    pub name: &'static str,
    lib: libloading::Library,
}

impl Impl {
    pub fn driver(&self) -> libloading::Symbol<'_, DriverFn> {
        unsafe { self.lib.get(b"driver\0") }
            .unwrap_or_else(|e| panic!("{}: no `driver`: {e}", self.name))
    }
    pub fn print_foo(&self) -> libloading::Symbol<'_, PrintFooFn> {
        unsafe { self.lib.get(b"print_foo\0") }
            .unwrap_or_else(|e| panic!("{}: no `print_foo`: {e}", self.name))
    }
    pub fn has(&self, sym: &[u8]) -> bool {
        let mut s = sym.to_vec();
        s.push(0);
        unsafe { self.lib.get::<*const ()>(&s) }.is_ok()
    }
}

/// The two loaded implementations: `(c, rust)`.
pub fn impls() -> &'static (Impl, Impl) {
    static I: OnceLock<(Impl, Impl)> = OnceLock::new();
    I.get_or_init(|| {
        let c = unsafe { libloading::Library::new(c_shared_lib()) }.expect("dlopen C .so");
        let r = unsafe { libloading::Library::new(rust_shared_lib()) }.expect("dlopen Rust .so");
        (
            Impl { name: "C", lib: c },
            Impl {
                name: "Rust",
                lib: r,
            },
        )
    })
}

/// Serializes everything that plays with fd 1 (`cargo test` runs the tests of
/// one binary on several threads).
fn fd1_lock() -> &'static Mutex<u64> {
    static L: OnceLock<Mutex<u64>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(0))
}

/// Restores fd 1 even if the captured closure panics.
struct Fd1Restore(c_int);

impl Drop for Fd1Restore {
    fn drop(&mut self) {
        unsafe {
            let _ = std::io::stdout().flush();
            fflush(std::ptr::null_mut());
            dup2(self.0, 1);
            close(self.0);
        }
    }
}

/// Runs `f` with fd 1 redirected into a temporary file and returns everything
/// that was written to it, flushing both libc's and Rust's `stdout` buffers.
///
/// NOTE: this must only be used from a **single-threaded** test binary
/// (`harness = false`), because `libtest`'s progress output would otherwise be
/// written to the redirected fd 1 by other threads.  See `tests/ffi_inproc.rs`.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let mut guard = fd1_lock().lock().unwrap();
    *guard += 1;
    let path = scratch_dir().join(format!("capture-{}-{}.out", std::process::id(), *guard));
    let file = std::fs::File::create(&path).expect("create capture file");
    {
        let _restore = unsafe {
            let _ = std::io::stdout().flush();
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1)");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2");
            Fd1Restore(saved)
        };
        f();
    }
    drop(file);
    let data = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    data
}

// ---------------------------------------------------------------------------
// Minimal test runner for the `harness = false` test binaries
// ---------------------------------------------------------------------------

/// Runs every `(name, check)` pair, reporting one line per check, and exits
/// with a non-zero status if any of them panicked.
pub fn run_checks(checks: &[(&str, fn())]) {
    let mut failed = Vec::new();
    println!("running {} checks", checks.len());
    for (name, f) in checks {
        print!("check {name} ... ");
        let _ = std::io::stdout().flush();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(()) => println!("ok"),
            Err(_) => {
                println!("FAILED");
                failed.push(*name);
            }
        }
    }
    if failed.is_empty() {
        println!("\nchecks result: ok. {} passed; 0 failed", checks.len());
    } else {
        println!(
            "\nchecks result: FAILED. {} passed; {} failed: {:?}",
            checks.len() - failed.len(),
            failed.len(),
            failed
        );
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Differential helpers
// ---------------------------------------------------------------------------

/// Calls `driver(x, y, b, z)` in both libraries and asserts byte-identical
/// stdout.  Both sides are invoked for a whole batch of argument tuples so
/// that one fd redirect covers many calls.
pub fn assert_driver_batch(cases: &[(u32, u32, u8, i32)], label: &str) {
    let (c, r) = impls();
    let c_out = capture_stdout(|| {
        let f = c.driver();
        for &(x, y, b, z) in cases {
            unsafe { f(x, y, b, z) };
        }
    });
    let r_out = capture_stdout(|| {
        let f = r.driver();
        for &(x, y, b, z) in cases {
            unsafe { f(x, y, b, z) };
        }
    });
    compare_lines(&c_out, &r_out, cases.len(), label, &|i| {
        format!("driver({}, {}, {}, {})", cases[i].0, cases[i].1, cases[i].2, cases[i].3)
    });
}

/// Same for `print_foo`, over raw `foo_t` byte images.
pub fn assert_print_foo_batch(cases: &[(u8, [u8; 3], i32)], label: &str) {
    let (c, r) = impls();
    let c_out = capture_stdout(|| {
        let f = c.print_foo();
        for &(bits, pad, z) in cases {
            let raw = RawFoo::new(bits, pad, z);
            unsafe { f(raw.as_ptr()) };
        }
    });
    let r_out = capture_stdout(|| {
        let f = r.print_foo();
        for &(bits, pad, z) in cases {
            let raw = RawFoo::new(bits, pad, z);
            unsafe { f(raw.as_ptr()) };
        }
    });
    compare_lines(&c_out, &r_out, cases.len(), label, &|i| {
        let (bits, pad, z) = cases[i];
        format!("print_foo({{bits=0x{bits:02x}, pad={pad:?}, z={z}}})")
    });
}

fn compare_lines(
    c_out: &[u8],
    r_out: &[u8],
    n: usize,
    label: &str,
    describe: &dyn Fn(usize) -> String,
) {
    if c_out == r_out {
        let lines = c_out.iter().filter(|&&b| b == b'\n').count();
        assert_eq!(lines, n, "[{label}] expected {n} output lines, got {lines}");
        return;
    }
    let cl: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
    let rl: Vec<&[u8]> = r_out.split(|&b| b == b'\n').collect();
    for i in 0..cl.len().max(rl.len()) {
        let a = cl.get(i).copied().unwrap_or(b"<missing>");
        let b = rl.get(i).copied().unwrap_or(b"<missing>");
        if a != b {
            panic!(
                "[{label}] divergence on case #{i} — {}\n  C   : {:?}\n  Rust: {:?}",
                describe(i.min(n.saturating_sub(1))),
                String::from_utf8_lossy(a),
                String::from_utf8_lossy(b),
            );
        }
    }
    panic!("[{label}] outputs differ but no differing line found");
}

// ---------------------------------------------------------------------------
// Out-of-process helpers (executables and dlopen'd `main`)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct Run {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr_empty: bool,
}

fn to_run(out: Output) -> Run {
    use std::os::unix::process::ExitStatusExt;
    Run {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout: out.stdout,
        stderr_empty: out.stderr.is_empty(),
    }
}

/// Runs a program with `input` on stdin and captures the result.
pub fn run_with_stdin(prog: &Path, args: &[&str], input: &[u8]) -> Run {
    let mut child = Command::new(prog)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", prog.display()));
    {
        let stdin = child.stdin.as_mut().unwrap();
        // The child may exit before consuming everything; a broken pipe here
        // is not a test failure.
        let _ = stdin.write_all(input);
        let _ = stdin.flush();
    }
    to_run(child.wait_with_output().expect("wait_with_output"))
}

/// The C executable vs the Rust executable on the same stdin.
pub fn assert_exe_same(input: &[u8], label: &str) {
    let c = run_with_stdin(&c_exe(), &[], input);
    let r = run_with_stdin(&rust_exe(), &[], input);
    assert_eq!(
        c, r,
        "[{label}] executables diverge for stdin {:?}",
        Preview(input)
    );
}

/// The C `.so`'s `main` vs the Rust `.so`'s `main`, each `dlopen`ed in a fresh
/// process, on the same stdin.
pub fn assert_so_main_same(input: &[u8], label: &str) {
    let cso = c_shared_lib();
    let rso = rust_shared_lib();
    let runner = so_runner();
    let c = run_with_stdin(&runner, &[cso.to_str().unwrap(), "main"], input);
    let r = run_with_stdin(&runner, &[rso.to_str().unwrap(), "main"], input);
    assert_eq!(
        c, r,
        "[{label}] .so `main` diverges for stdin {:?}",
        Preview(input)
    );
}

/// Runs a `so_runner` sub-command against both shared objects and asserts the
/// full result (exit code, signal, stdout) is identical.
pub fn assert_runner_same(args: &[&str], input: &[u8], label: &str) {
    let cso = c_shared_lib();
    let rso = rust_shared_lib();
    let runner = so_runner();
    let mut cargs = vec![cso.to_str().unwrap()];
    cargs.extend_from_slice(args);
    let mut rargs = vec![rso.to_str().unwrap()];
    rargs.extend_from_slice(args);
    let c = run_with_stdin(&runner, &cargs, input);
    let r = run_with_stdin(&runner, &rargs, input);
    assert_eq!(
        (c.code, c.signal, &c.stdout),
        (r.code, r.signal, &r.stdout),
        "[{label}] runner {args:?} diverges"
    );
}

pub struct Preview<'a>(pub &'a [u8]);

impl std::fmt::Debug for Preview<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.0.len();
        let head = &self.0[..n.min(120)];
        write!(f, "{:?}", String::from_utf8_lossy(head))?;
        if n > head.len() {
            write!(f, "…(+{} bytes)", n - head.len())?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed, reproducible runs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}

pub const SEED: u64 = 0x5EED_1234;
