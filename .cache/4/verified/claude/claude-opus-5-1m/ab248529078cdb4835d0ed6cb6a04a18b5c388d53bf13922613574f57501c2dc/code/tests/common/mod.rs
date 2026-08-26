//! Shared helpers for the C-vs-Rust differential tests.
//!
//! * builds the C translation unit (`c_src/src/main.c`, never modified) both as
//!   a shared library and as an executable,
//!   locates/builds the Rust `cdylib` and executable,
//! * loads both `.so`s with `libloading` and calls their exported C-ABI symbols
//!   (`static_sum`, `main`) — the Rust side is *never* called directly, always
//!   through the `.so` exports,
//! * captures whatever the called `main` writes to fd 1.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, OsString};
use std::io::Write;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------- paths -----

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_source() -> PathBuf {
    manifest_dir().join("c_src/src/main.c")
}

/// Per-test-binary scratch directory (inside `target/`, never inside `c_src/`).
pub fn work_dir(tag: &str) -> PathBuf {
    let d = manifest_dir().join("target").join("cdiff").join(tag);
    std::fs::create_dir_all(&d).expect("create work dir");
    d
}

fn run_ok(cmd: &mut Command) {
    let out = cmd.output().unwrap_or_else(|e| panic!("failed to spawn {cmd:?}: {e}"));
    assert!(
        out.status.success(),
        "command {cmd:?} failed: {}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
}

fn build_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Builds `out` (once, race-free) if it is missing or older than `sources`.
///
/// `build` receives a temporary destination which is `rename`d into place, so
/// concurrent tests never observe a half-written artifact.
fn ensure_artifact<F>(out: PathBuf, sources: &[PathBuf], build: F) -> PathBuf
where
    F: FnOnce(&Path),
{
    let _guard = build_lock().lock().unwrap_or_else(|e| e.into_inner());
    if is_fresh(&out, sources) {
        return out;
    }
    let n = INSTANCE.fetch_add(1, Ordering::SeqCst);
    let tmp = out.with_extension(format!("tmp.{}.{}", std::process::id(), n));
    build(&tmp);
    std::fs::rename(&tmp, &out).unwrap_or_else(|e| panic!("rename {tmp:?} -> {out:?}: {e}"));
    out
}

/// `gcc -shared -fPIC c_src/src/main.c` — the C code under test as a `.so`.
pub fn c_so(tag: &str) -> PathBuf {
    ensure_artifact(
        work_dir(tag).join("libcdriver.so"),
        &[c_source()],
        |tmp| {
            run_ok(
                Command::new("gcc")
                    .arg("-shared")
                    .arg("-fPIC")
                    .arg("-o")
                    .arg(tmp)
                    .arg(c_source()),
            )
        },
    )
}

/// The C executable. Prefers the CMake artefact (`c_src/build/driver`, built
/// exactly as `c_src/CMakeLists.txt` prescribes); falls back to a plain `gcc`
/// build with the same (default, unoptimised) flags.
pub fn c_exe(tag: &str) -> PathBuf {
    let cmake_built = manifest_dir().join("c_src/build/driver");
    if cmake_built.is_file() {
        return cmake_built;
    }
    ensure_artifact(work_dir(tag).join("cdriver"), &[c_source()], |tmp| {
        run_ok(Command::new("gcc").arg("-o").arg(tmp).arg(c_source()))
    })
}

/// The Rust `cdylib`. Uses the cargo artefact when it is present and current
/// (`cargo build --lib`, optionally `--release`, see `run_diff_tests.sh`), and
/// otherwise builds it with `rustc` using the same crate type.
pub fn rust_so() -> PathBuf {
    if let Some(p) = std::env::var_os("RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "RUST_SO={p:?} does not exist");
        return p;
    }

    let sources = vec![
        manifest_dir().join("src/lib.rs"),
        manifest_dir().join("src/logic.rs"),
    ];

    // target/<profile>/libdriver.so, next to the test executable's bin dir.
    let bin_dir = PathBuf::from(env!("CARGO_BIN_EXE_driver"))
        .parent()
        .unwrap()
        .to_path_buf();
    let cargo_artifact = bin_dir.join("libdriver.so");
    if is_fresh(&cargo_artifact, &sources) {
        return cargo_artifact;
    }

    ensure_artifact(
        work_dir("rustlib").join("libdriver.so"),
        &sources,
        |tmp| {
            run_ok(
                Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into()))
                    .arg("--edition=2021")
                    .arg("--crate-type=cdylib")
                    .arg("--crate-name=driver")
                    .arg("-o")
                    .arg(tmp)
                    .arg(manifest_dir().join("src/lib.rs")),
            )
        },
    )
}

/// The Rust executable built by cargo for this test run.
pub fn rust_exe() -> PathBuf {
    if let Some(p) = std::env::var_os("RUST_EXE") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_BIN_EXE_driver"))
}

fn mtime(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

fn is_fresh(artifact: &Path, sources: &[PathBuf]) -> bool {
    let Some(a) = mtime(artifact) else { return false };
    sources.iter().all(|s| match mtime(s) {
        Some(t) => t <= a,
        None => true,
    })
}

// ------------------------------------------------------------ .so loading ---

static INSTANCE: AtomicU64 = AtomicU64::new(0);

/// A freshly `dlopen`ed copy of a library.
///
/// The copy gets a unique file name, because `dlopen`ing the *same* path twice
/// returns the same (ref-counted) handle and would therefore share the
/// `static int sum` state. A unique path yields a pristine instance.
pub struct Lib {
    lib: libloading::Library,
    pub path: PathBuf,
}

impl Lib {
    pub fn open_fresh(original: &Path, tag: &str) -> Lib {
        let n = INSTANCE.fetch_add(1, Ordering::SeqCst);
        let name = original.file_name().unwrap().to_string_lossy().into_owned();
        let dir = work_dir(tag).join("instances");
        std::fs::create_dir_all(&dir).unwrap();
        let copy = dir.join(format!("{}.{}.{}.so", name, std::process::id(), n));
        std::fs::copy(original, &copy)
            .unwrap_or_else(|e| panic!("copy {original:?} -> {copy:?}: {e}"));
        let lib = unsafe { libloading::Library::new(&copy) }
            .unwrap_or_else(|e| panic!("dlopen {copy:?}: {e}"));
        Lib { lib, path: copy }
    }

    /// `dlsym` probe: is `name` (NUL-terminated) resolvable in this `.so`?
    pub fn has_symbol(&self, name: &[u8]) -> bool {
        unsafe {
            self.lib
                .get::<*mut std::ffi::c_void>(name)
                .is_ok()
        }
    }

    /// `int static_sum(int update)` from the loaded `.so`.
    pub fn static_sum(&self, update: i32) -> i32 {
        unsafe {
            let f: libloading::Symbol<unsafe extern "C" fn(c_int) -> c_int> =
                self.lib.get(b"static_sum\0").expect("symbol static_sum");
            f(update)
        }
    }

    /// `int main(int argc, char **argv)` from the loaded `.so`.
    ///
    /// `argc` is passed verbatim (so bogus values can be tested) while `args`
    /// provides the NUL-terminated `argv` array. Returns `(return value,
    /// bytes written to fd 1)`.
    pub fn call_main(&self, argc: i32, args: &[Vec<u8>]) -> (i32, Vec<u8>) {
        let mut storage: Vec<Vec<u8>> = args
            .iter()
            .map(|a| {
                let mut v = a.clone();
                v.push(0);
                v
            })
            .collect();
        let mut argv: Vec<*mut c_char> = storage
            .iter_mut()
            .map(|s| s.as_mut_ptr() as *mut c_char)
            .collect();
        argv.push(std::ptr::null_mut()); // C guarantees argv[argc] == NULL

        let f: libloading::Symbol<unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int> =
            unsafe { self.lib.get(b"main\0") }.expect("symbol main");

        let (rc, out) = capture_fd1(|| unsafe { f(argc, argv.as_mut_ptr()) });
        (rc, out)
    }

    /// Convenience: `main` with `argc` derived from `args`.
    pub fn call_main_auto(&self, args: &[Vec<u8>]) -> (i32, Vec<u8>) {
        self.call_main(args.len() as i32, args)
    }

    /// `main(argc, NULL)`. The C code only dereferences `argv` when
    /// `argc == 2`, so for every other `argc` a null `argv` is a perfectly
    /// well-defined input that must be rejected with the argc error message.
    pub fn call_main_null_argv(&self, argc: i32) -> (i32, Vec<u8>) {
        assert_ne!(argc, 2, "argc == 2 with argv == NULL is UB in the C code");
        let f: libloading::Symbol<unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int> =
            unsafe { self.lib.get(b"main\0") }.expect("symbol main");
        capture_fd1(|| unsafe { f(argc, std::ptr::null_mut()) })
    }
}

impl Drop for Lib {
    fn drop(&mut self) {
        // Unlinking a mapped library is fine on Linux; keeps `target/` tidy.
        let _ = std::fs::remove_file(&self.path);
    }
}

/// The C `.so` and the Rust `.so`, both pristine, ready for lock-step calls.
pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

pub fn fresh_pair(tag: &str) -> Pair {
    Pair {
        c: Lib::open_fresh(&c_so(tag), tag),
        rust: Lib::open_fresh(&rust_so(), tag),
    }
}

impl Pair {
    /// Calls `main(argc, argv)` in the C `.so` and in the Rust `.so` and
    /// asserts that the return value and every byte written to stdout match.
    pub fn assert_main_same(&self, argc: i32, args: &[Vec<u8>], ctx: &str) -> (i32, Vec<u8>) {
        let (rc_c, out_c) = self.c.call_main(argc, args);
        let (rc_r, out_r) = self.rust.call_main(argc, args);
        assert_eq!(
            rc_c, rc_r,
            "main() return value differs for {ctx} (argc = {argc}, argv = {:?})",
            show_args(args)
        );
        assert_eq!(
            out_c,
            out_r,
            "main() stdout differs for {ctx} (argc = {argc}, argv = {:?})\n C: {:?}\n R: {:?}",
            show_args(args),
            String::from_utf8_lossy(&out_c),
            String::from_utf8_lossy(&out_r),
        );
        (rc_c, out_c)
    }

    /// Same, with `argc` derived from `args`.
    pub fn assert_main_same_auto(&self, args: &[Vec<u8>], ctx: &str) -> (i32, Vec<u8>) {
        self.assert_main_same(args.len() as i32, args, ctx)
    }

    /// Same, for `main(argc, NULL)`.
    pub fn assert_main_same_null_argv(&self, argc: i32, ctx: &str) -> (i32, Vec<u8>) {
        let (rc_c, out_c) = self.c.call_main_null_argv(argc);
        let (rc_r, out_r) = self.rust.call_main_null_argv(argc);
        assert_eq!(rc_c, rc_r, "main(argc = {argc}, NULL) return value differs ({ctx})");
        assert_eq!(
            out_c,
            out_r,
            "main(argc = {argc}, NULL) stdout differs ({ctx})\n C: {:?}\n R: {:?}",
            String::from_utf8_lossy(&out_c),
            String::from_utf8_lossy(&out_r)
        );
        (rc_c, out_c)
    }
}

pub fn show_args(args: &[Vec<u8>]) -> Vec<String> {
    args.iter()
        .map(|a| format!("{:?}", String::from_utf8_lossy(a)))
        .collect()
}

/// Builds the `argv` for a one-operand invocation: `["driver", arg]`.
pub fn argv1(arg: &[u8]) -> Vec<Vec<u8>> {
    vec![b"driver".to_vec(), arg.to_vec()]
}

// -------------------------------------------------------- stdout capture ----

fn fd_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Runs `f` with file descriptor 1 redirected into a temporary file and returns
/// `f`'s result together with everything that was written to it.
///
/// Both C `printf` (libc `stdout` buffer) and the Rust `.so`'s `std::io::stdout`
/// end up on fd 1, so this captures either implementation faithfully.
///
/// NOTE: fd 1 is process-wide, and libtest also prints its progress lines to
/// it, so every test binary that captures must contain a single `#[test]`
/// function (see `tests/ffi_main.rs`, `tests/ffi_errors.rs`). The guard below
/// serialises captures within a process and the assertion catches accidental
/// contamination should that rule ever be broken.
pub fn capture_fd1<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    let _guard = fd_lock().lock().unwrap_or_else(|e| e.into_inner());

    let n = INSTANCE.fetch_add(1, Ordering::SeqCst);
    let tmp = work_dir("capture").join(format!("out.{}.{}.bin", std::process::id(), n));

    // Flush anything still pending so it does not land in the capture file.
    let _ = std::io::stdout().flush();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let file = std::fs::File::create(&tmp).expect("create capture file");
    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { libc::dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let result = f();

    // Flush the callee's buffers *while* fd 1 is still redirected.
    unsafe { libc::fflush(std::ptr::null_mut()) };
    let _ = std::io::stdout().flush();

    assert!(unsafe { libc::dup2(saved, 1) } >= 0, "restore dup2 failed");
    unsafe { libc::close(saved) };
    drop(file);

    let bytes = std::fs::read(&tmp).expect("read capture file");
    let _ = std::fs::remove_file(&tmp);
    assert!(
        !contains(&bytes, b"running ") && !contains(&bytes, b" ... ok"),
        "the fd-1 capture was contaminated by libtest output; the capturing \
         test binary must contain exactly one #[test] function.\ncaptured: {:?}",
        String::from_utf8_lossy(&bytes)
    );
    (result, bytes)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w == needle)
}

// ------------------------------------------------------------ subprocess ----

#[derive(Debug, PartialEq, Eq)]
pub struct RunOut {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub fn run_exe_full(exe: &Path, args: &[Vec<u8>], argv0: Option<&[u8]>, env: Option<&[(&str, &str)]>) -> RunOut {
    use std::os::unix::process::CommandExt;
    use std::os::unix::process::ExitStatusExt;

    let mut cmd = Command::new(exe);
    for a in args {
        cmd.arg(OsString::from_vec(a.clone()));
    }
    if let Some(a0) = argv0 {
        cmd.arg0(OsString::from_vec(a0.to_vec()));
    }
    if let Some(env) = env {
        cmd.env_clear();
        for (k, v) in env {
            cmd.env(k, v);
        }
    }
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run {exe:?}: {e}"));
    RunOut {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

pub fn run_exe(exe: &Path, args: &[Vec<u8>]) -> RunOut {
    run_exe_full(exe, args, None, None)
}

/// Runs the C and the Rust executable with the same arguments and asserts that
/// stdout, stderr and the exit status match byte for byte.
pub fn assert_exe_same(tag: &str, args: &[Vec<u8>]) {
    let c = run_exe(&c_exe(tag), args);
    let r = run_exe(&rust_exe(), args);
    assert_same(&c, &r, args);
}

pub fn assert_same(c: &RunOut, r: &RunOut, args: &[Vec<u8>]) {
    let show = |a: &[Vec<u8>]| {
        a.iter()
            .map(|x| String::from_utf8_lossy(x).into_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        c.stdout,
        r.stdout,
        "stdout differs for args {:?}\n C: {:?}\n R: {:?}",
        show(args),
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(c.stderr, r.stderr, "stderr differs for args {:?}", show(args));
    assert_eq!(c.code, r.code, "exit code differs for args {:?}", show(args));
    assert_eq!(c.signal, r.signal, "signal differs for args {:?}", show(args));
}

// ------------------------------------------------------------------- rng ----

/// SplitMix64 — deterministic, seeded, no external crate needed.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_C0DE_1234_5678;

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }
}
