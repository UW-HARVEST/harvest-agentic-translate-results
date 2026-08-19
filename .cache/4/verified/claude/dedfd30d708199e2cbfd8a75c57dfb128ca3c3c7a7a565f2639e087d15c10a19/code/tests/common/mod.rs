// Shared plumbing for the C <-> Rust differential tests.
//
// * builds the C sources (unmodified) into a shared object and an executable,
// * locates the Rust shared object (`examples/libcapi.so`) and the Rust
//   executable (`driver`), building them if necessary,
// * loads BOTH shared objects with `libloading` and resolves the exported
//   symbols, so every call in every test goes through the real C ABI export
//   (never through a direct Rust call),
// * provides a deterministic PRNG and a stdout capture helper.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CString};
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------- paths/builds

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` of the current test run.
pub fn profile_dir() -> PathBuf {
    // current_exe is target/<profile>/deps/<test>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

/// Scratch directory for artifacts produced by the tests.
pub fn work_dir() -> PathBuf {
    let dir = manifest_dir().join("target").join("difftest");
    std::fs::create_dir_all(&dir).expect("create work dir");
    dir
}

fn newer_than(a: &Path, b: &Path) -> bool {
    let (Ok(ma), Ok(mb)) = (std::fs::metadata(a), std::fs::metadata(b)) else {
        return true;
    };
    match (ma.modified(), mb.modified()) {
        (Ok(ta), Ok(tb)) => ta > tb,
        _ => true,
    }
}

fn unique_tmp(dir: &Path, stem: &str) -> PathBuf {
    dir.join(format!("{stem}.{}.tmp", std::process::id()))
}

/// Each artifact is prepared at most once per test process, even though the test
/// threads run in parallel.
fn once<F: FnOnce() -> PathBuf>(cell: &'static std::sync::OnceLock<PathBuf>, f: F) -> PathBuf {
    cell.get_or_init(f).clone()
}

/// The C source, compiled unchanged into a shared object.
pub fn c_so() -> PathBuf {
    static CELL: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    once(&CELL, c_so_build)
}

fn c_so_build() -> PathBuf {
    let src = manifest_dir().join("c_src").join("src").join("main.c");
    let out = work_dir().join("libcdriver.so");
    if !out.exists() || newer_than(&src, &out) {
        let tmp = unique_tmp(&work_dir(), "libcdriver.so");
        let status = Command::new("cc")
            .args(["-shared", "-fPIC", "-o"])
            .arg(&tmp)
            .arg(&src)
            .status()
            .expect("run cc");
        assert!(status.success(), "compiling {} failed", src.display());
        std::fs::rename(&tmp, &out).expect("install libcdriver.so");
    }
    out
}

/// The C executable. Prefers the CMake build described in c_src/CMakeLists.txt
/// (`add_executable(driver src/main.c)`) and falls back to a direct `cc` call
/// with the same (default, no -O) flags.
pub fn c_exe() -> PathBuf {
    static CELL: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    once(&CELL, c_exe_build)
}

fn c_exe_build() -> PathBuf {
    let src = manifest_dir().join("c_src").join("src").join("main.c");
    let cmake_built = manifest_dir().join("c_src").join("build").join("driver");
    if cmake_built.exists() && !newer_than(&src, &cmake_built) {
        return cmake_built;
    }
    let out = work_dir().join("cdriver");
    if !out.exists() || newer_than(&src, &out) {
        let tmp = unique_tmp(&work_dir(), "cdriver");
        let status = Command::new("cc")
            .arg("-o")
            .arg(&tmp)
            .arg(&src)
            .status()
            .expect("run cc");
        assert!(status.success(), "compiling {} failed", src.display());
        std::fs::rename(&tmp, &out).expect("install cdriver");
    }
    out
}

fn cargo_build(args: &[&str]) {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(manifest_dir());
    cmd.arg("build").arg("--offline").args(args);
    // Build into the same profile directory the running test lives in.
    let profile = profile_dir();
    match profile.file_name().and_then(|s| s.to_str()) {
        Some("debug") | None => {}
        Some("release") => {
            cmd.arg("--release");
        }
        Some(other) => {
            cmd.arg("--profile").arg(other);
        }
    }
    // Keep the sub-build out of the parent's target lock contention path is not
    // needed: cargo releases the build lock before running test binaries.
    let status = cmd.status().expect("run cargo build");
    assert!(status.success(), "cargo build {args:?} failed");
}

/// Rust sources the shared object / executable are built from.
fn rust_sources() -> Vec<PathBuf> {
    let m = manifest_dir();
    vec![
        m.join("src").join("lib.rs"),
        m.join("src").join("main.rs"),
        m.join("examples").join("capi.rs"),
        m.join("Cargo.toml"),
    ]
}

fn is_stale(out: &Path) -> bool {
    !out.exists() || rust_sources().iter().any(|src| newer_than(src, out))
}

/// The Rust shared object exporting the same C ABI as the C `.so`.
///
/// Rebuilt whenever a Rust source is newer than the artifact: `cargo test --test
/// <name>` does not necessarily refresh example targets, and silently loading a
/// stale `.so` would make the differential tests meaningless.
pub fn rust_so() -> PathBuf {
    static CELL: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    once(&CELL, || {
        let out = profile_dir().join("examples").join("libcapi.so");
        // `cargo build` is the authority on freshness; the mtime comparison only
        // decides whether it is worth asking it.
        if is_stale(&out) {
            cargo_build(&["--example", "capi"]);
        }
        assert!(out.exists(), "missing {}", out.display());
        out
    })
}

/// The Rust executable (counterpart of the C `driver` program).
pub fn rust_exe() -> PathBuf {
    static CELL: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    once(&CELL, || {
        let out = profile_dir().join("driver");
        if is_stale(&out) {
            cargo_build(&["--bin", "driver"]);
        }
        assert!(out.exists(), "missing {}", out.display());
        out
    })
}

// ------------------------------------------------------------------ FFI loading

pub type StaticAliasFn = unsafe extern "C" fn(*mut c_int) -> *mut c_int;
pub type MainFn = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

pub struct Impl {
    pub name: &'static str,
    pub static_alias: StaticAliasFn,
    pub main: MainFn,
    _lib: libloading::Library,
}

impl Impl {
    pub fn load(path: &Path, name: &'static str) -> Impl {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
            let static_alias = *lib
                .get::<StaticAliasFn>(b"static_alias\0")
                .unwrap_or_else(|e| panic!("dlsym static_alias in {}: {e}", path.display()));
            let main = *lib
                .get::<MainFn>(b"main\0")
                .unwrap_or_else(|e| panic!("dlsym main in {}: {e}", path.display()));
            Impl {
                name,
                static_alias,
                main,
                _lib: lib,
            }
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

/// Loads a *fresh* copy of both shared objects.
///
/// Each `.so` is copied to a unique path first: `dlopen` refcounts by file
/// identity, so distinct paths give distinct mappings and therefore a pristine
/// `static int inner = 1;` per scenario — exactly like a freshly started
/// process.
pub fn load_pair(tag: &str) -> Pair {
    let inst = work_dir().join("inst");
    std::fs::create_dir_all(&inst).expect("create inst dir");
    let pid = std::process::id();
    let c_dst = inst.join(format!("c-{tag}-{pid}.so"));
    let r_dst = inst.join(format!("r-{tag}-{pid}.so"));
    std::fs::copy(c_so(), &c_dst).expect("copy C .so");
    std::fs::copy(rust_so(), &r_dst).expect("copy Rust .so");
    let pair = Pair {
        c: Impl::load(&c_dst, "C"),
        rust: Impl::load(&r_dst, "Rust"),
    };
    // The images stay mapped after the files are unlinked (POSIX), so the
    // scratch copies do not accumulate.
    std::fs::remove_file(&c_dst).ok();
    std::fs::remove_file(&r_dst).ok();
    pair
}

// ------------------------------------------------------------------ argv arrays

/// A C `argv` array: NUL-terminated strings plus the terminating NULL pointer.
pub struct Argv {
    storage: Vec<CString>,
    ptrs: Vec<*mut c_char>,
}

impl Argv {
    pub fn new<T: AsRef<[u8]>>(args: &[T]) -> Argv {
        let storage: Vec<CString> = args
            .iter()
            .map(|a| CString::new(a.as_ref().to_vec()).expect("no interior NUL"))
            .collect();
        let mut ptrs: Vec<*mut c_char> = storage.iter().map(|s| s.as_ptr() as *mut c_char).collect();
        ptrs.push(std::ptr::null_mut());
        Argv { storage, ptrs }
    }

    pub fn argc(&self) -> c_int {
        self.storage.len() as c_int
    }

    pub fn ptr(&mut self) -> *mut *mut c_char {
        self.ptrs.as_mut_ptr()
    }
}

// -------------------------------------------------------------- stdout capture

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Runs `f` with file descriptor 1 redirected into a temporary file and returns
/// its result together with the bytes that were written to stdout.
///
/// Flushes the C stdio buffers (`fflush(NULL)`) before restoring the descriptor,
/// so `printf` output produced by the loaded C `.so` is captured even though the
/// stream is fully buffered when stdout is a file.
///
/// Not thread safe (it changes a process-wide descriptor): the test binaries
/// that use it run all their scenarios from a single `#[test]`.
pub fn with_captured_stdout<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    use std::io::Write;
    use std::os::unix::io::AsRawFd;

    let path = work_dir().join(format!("stdout-{}.bin", std::process::id()));
    let file = std::fs::File::create(&path).expect("create capture file");

    std::io::stdout().flush().ok();
    unsafe {
        fflush(std::ptr::null_mut());
    }

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let result = f();

    unsafe {
        fflush(std::ptr::null_mut());
    }
    std::io::stdout().flush().ok();
    assert!(unsafe { dup2(saved, 1) } >= 0, "restore dup2 failed");
    unsafe {
        close(saved);
    }
    drop(file);

    let bytes = std::fs::read(&path).expect("read capture file");
    std::fs::remove_file(&path).ok();
    (result, bytes)
}

/// Calls the exported `main` of `imp` and captures stdout and the return value.
pub fn call_main(imp: &Impl, argc: c_int, argv: &mut Argv) -> (c_int, Vec<u8>) {
    let f = imp.main;
    let p = argv.ptr();
    with_captured_stdout(|| unsafe { f(argc, p) })
}

// -------------------------------------------------------------------- PRNG etc.

/// SplitMix64 — deterministic, seeded, no external dependency.
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\n', "\\n")
}
