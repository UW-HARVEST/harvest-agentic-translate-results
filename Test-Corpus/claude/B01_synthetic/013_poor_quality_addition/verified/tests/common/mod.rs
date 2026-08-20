//! Shared differential-test harness.
//!
//! Both implementations are loaded as **shared objects** through `libloading`
//! and driven exclusively through their exported C symbols — the Rust functions
//! are never called directly, so the `#[no_mangle]` export wrappers in
//! `src/lib.rs` are part of what is under test.
//!
//! * C  side: `c_build/libdriver_c.so`   (built from `c_src/src/main.c`)
//! * Rust side: `target/<profile>/libdriver.so` (the crate's `cdylib` target)
//!
//! Output is compared by redirecting fd 1 to a temp file around each call, so
//! the comparison is on the exact bytes each implementation writes to stdout.

#![allow(dead_code)]

use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::Write;
use std::os::raw::{c_char, c_int};
use std::os::unix::io::AsRawFd;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::Library;

/// Fixed seed so every randomized (property-style) row is reproducible.
pub const SEED: u64 = 0x2026_0818_C0FF_EE01;

// ---------------------------------------------------------------------------
// paths / building the C shared object
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn mtime(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).ok()?.modified().ok()
}

/// Compile `c_src/src/main.c` into `c_build/libdriver_c.so` if needed.
///
/// `c_src/CMakeLists.txt` only declares `add_executable`, and we must not modify
/// anything under `c_src/`, so the shared-library flavour of the very same
/// translation unit is produced out-of-tree with the same compiler.
fn ensure_c_shared_object() -> PathBuf {
    let root = manifest_dir();
    let src = root.join("c_src/src/main.c");
    let out_dir = root.join("c_build");
    let so = out_dir.join("libdriver_c.so");
    std::fs::create_dir_all(&out_dir).expect("create c_build");

    let stale = match (mtime(&so), mtime(&src)) {
        (Some(a), Some(b)) => a < b,
        _ => true,
    };
    if stale {
        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
        let out = std::process::Command::new(&cc)
            .args(["-shared", "-fPIC", "-O2", "-o"])
            .arg(&so)
            .arg(&src)
            .output()
            .unwrap_or_else(|e| panic!("failed to run {cc}: {e}"));
        assert!(
            out.status.success(),
            "compiling the C shared object failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    so
}

/// Compile the C executable exactly as `c_src/CMakeLists.txt` does, reusing the
/// cmake output when it is present.
pub fn c_executable() -> PathBuf {
    let root = manifest_dir();
    let src = root.join("c_src/src/main.c");
    let cmake_built = root.join("c_src/build/driver");
    if mtime(&cmake_built).is_some() {
        return cmake_built;
    }
    let out_dir = root.join("c_build");
    std::fs::create_dir_all(&out_dir).expect("create c_build");
    let exe = out_dir.join("driver_c");
    let stale = match (mtime(&exe), mtime(&src)) {
        (Some(a), Some(b)) => a < b,
        _ => true,
    };
    if stale {
        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
        let out = std::process::Command::new(&cc)
            .args(["-O2", "-o"])
            .arg(&exe)
            .arg(&src)
            .output()
            .unwrap_or_else(|e| panic!("failed to run {cc}: {e}"));
        assert!(
            out.status.success(),
            "compiling the C executable failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    exe
}

/// Directory holding the current test binary's build artifacts
/// (`target/debug` or `target/release`).
fn artifact_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-binary>
    exe.parent()
        .and_then(|p| p.parent())
        .expect("artifact dir")
        .to_path_buf()
}

fn newest_lib_source_mtime() -> std::time::SystemTime {
    let root = manifest_dir();
    ["src/lib.rs", "src/driver.rs"]
        .iter()
        .filter_map(|p| mtime(&root.join(p)))
        .max()
        .expect("lib sources")
}

/// Path of the Rust `cdylib` under test.
///
/// Prefers the artifact cargo built (`cargo build` / `cargo build --release`).
/// `cargo test` alone does not build a `cdylib`-only lib target, so as a
/// self-contained fallback the shared object is produced with a direct `rustc`
/// call — the crate has no dependencies, so that is exactly equivalent and it
/// avoids a nested `cargo` invocation (which would block on the target lock).
pub fn rust_shared_object() -> PathBuf {
    let dir = artifact_dir();
    let src_time = newest_lib_source_mtime();
    let candidates = [
        dir.join("libdriver.so"),
        manifest_dir().join("target/release/libdriver.so"),
        manifest_dir().join("target/debug/libdriver.so"),
    ];
    for c in candidates.iter() {
        if let Some(t) = mtime(c) {
            if t >= src_time {
                return c.clone();
            }
        }
    }
    build_rust_shared_object_with_rustc()
}

fn build_rust_shared_object_with_rustc() -> PathBuf {
    let root = manifest_dir();
    let out_dir = artifact_dir().join("diff_so");
    std::fs::create_dir_all(&out_dir).expect("create diff_so dir");
    let out = out_dir.join("libdriver.so");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let res = std::process::Command::new(&rustc)
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "cdylib",
            "--crate-name",
            "driver",
            "-O",
            "-o",
        ])
        .arg(&out)
        .arg(root.join("src/lib.rs"))
        .output()
        .unwrap_or_else(|e| panic!("failed to run {rustc}: {e}"));
    assert!(
        res.status.success(),
        "building libdriver.so with rustc failed:\n{}",
        String::from_utf8_lossy(&res.stderr)
    );
    out
}

/// Path of the Rust executable under test.
pub fn rust_executable() -> PathBuf {
    let dir = artifact_dir();
    let candidates = [
        dir.join("driver"),
        manifest_dir().join("target/debug/driver"),
        manifest_dir().join("target/release/driver"),
    ];
    for c in candidates.iter() {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("rust `driver` executable not found (looked in {:?})", candidates);
}

// ---------------------------------------------------------------------------
// the loaded APIs
// ---------------------------------------------------------------------------

pub type FnPrintLine = unsafe extern "C" fn(*const c_char);
pub type FnPrintIntLine = unsafe extern "C" fn(c_int);
pub type FnVoid = unsafe extern "C" fn();
pub type FnMain = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

/// The five exported C symbols of one implementation.
#[derive(Clone, Copy)]
pub struct Api {
    pub name: &'static str,
    pub print_line: FnPrintLine,
    pub print_int_line: FnPrintIntLine,
    pub bad: FnVoid,
    pub good: FnVoid,
    pub main: FnMain,
}

impl Api {
    /// Convenience: `printLine` with a Rust byte string (NUL is appended here,
    /// exactly like a C string literal).
    pub fn print_line_bytes(&self, bytes: &[u8]) {
        let c = CString::new(bytes).expect("interior NUL");
        unsafe { (self.print_line)(c.as_ptr()) }
    }
    pub fn print_line_null(&self) {
        unsafe { (self.print_line)(std::ptr::null()) }
    }
    pub fn call_main(&self, argc: c_int, argv: *mut *mut c_char) -> c_int {
        unsafe { (self.main)(argc, argv) }
    }
}

fn load(path: &Path) -> &'static Library {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
    Box::leak(Box::new(lib))
}

unsafe fn sym<T: Copy>(lib: &'static Library, name: &str) -> T {
    let s: libloading::Symbol<T> = lib
        .get(CString::new(name).unwrap().as_bytes_with_nul())
        .unwrap_or_else(|e| panic!("missing symbol `{name}`: {e}"));
    *s
}

fn api_from(name: &'static str, path: &Path) -> Api {
    let lib = load(path);
    unsafe {
        Api {
            name,
            print_line: sym(lib, "printLine"),
            print_int_line: sym(lib, "printIntLine"),
            bad: sym(lib, "bad"),
            good: sym(lib, "good"),
            main: sym(lib, "main"),
        }
    }
}

pub fn c_api() -> Api {
    static A: OnceLock<Api> = OnceLock::new();
    *A.get_or_init(|| api_from("C", &ensure_c_shared_object()))
}

pub fn rust_api() -> Api {
    static A: OnceLock<Api> = OnceLock::new();
    *A.get_or_init(|| api_from("Rust", &rust_shared_object()))
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Total number of `.so` invocations captured (C + Rust), for reporting.
pub static CAPTURES: AtomicU64 = AtomicU64::new(0);

fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

fn flush_everything() {
    let _ = std::io::stdout().flush();
    // Flush every C stream (the C .so's printf/puts are fully buffered when fd 1
    // is a file).  Same libc instance as the one the dlopen'd library uses.
    unsafe { libc::fflush(std::ptr::null_mut()) };
}

/// Run `f` with fd 1 redirected to a fresh temp file; return everything written
/// to stdout plus `f`'s value.
pub fn capture_ret<T, F: FnOnce() -> T>(f: F) -> (Vec<u8>, T) {
    static N: AtomicU64 = AtomicU64::new(0);
    CAPTURES.fetch_add(1, Ordering::Relaxed);
    let guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}.out",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));

    flush_everything();

    let file = File::create(&path).expect("create capture file");
    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { libc::dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");
    drop(file);

    let result = std::panic::catch_unwind(AssertUnwindSafe(f));

    flush_everything();
    unsafe {
        libc::dup2(saved, 1);
        libc::close(saved);
    }
    drop(guard);

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);

    match result {
        Ok(v) => (bytes, v),
        Err(p) => std::panic::resume_unwind(p),
    }
}

pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    capture_ret(f).0
}

// ---------------------------------------------------------------------------
// comparison helpers
// ---------------------------------------------------------------------------

fn escape(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b.iter().take(160) {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    if b.len() > 160 {
        s.push_str(&format!("... (+{} bytes)", b.len() - 160));
    }
    s
}

pub fn assert_bytes_eq(label: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    let first = c
        .iter()
        .zip(r.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| c.len().min(r.len()));
    panic!(
        "stdout mismatch [{label}]\n  C   len={} : {}\n  Rust len={} : {}\n  first difference at byte {first}",
        c.len(),
        escape(c),
        r.len(),
        escape(r),
        );
}

/// Run the same closure against both implementations and require identical
/// stdout bytes *and* identical return values.
pub fn diff_with<T: PartialEq + std::fmt::Debug, F: Fn(Api) -> T>(label: &str, f: F) {
    let (c_out, c_ret) = capture_ret(|| f(c_api()));
    let (r_out, r_ret) = capture_ret(|| f(rust_api()));
    assert!(
        c_ret == r_ret,
        "return value mismatch [{label}]: C={c_ret:?} Rust={r_ret:?}"
    );
    assert_bytes_eq(label, &c_out, &r_out);
}

/// Run the same closure against both implementations and require identical
/// stdout bytes.
pub fn diff<F: Fn(Api)>(label: &str, f: F) {
    diff_with(label, |api| {
        f(api);
        0u8
    })
}

// ---------------------------------------------------------------------------
// argv construction
// ---------------------------------------------------------------------------

/// Owns a C `argv` array (NULL-terminated, as the real `main` receives).
pub struct Argv {
    _owned: Vec<CString>,
    ptrs: Vec<*mut c_char>,
}

impl Argv {
    pub fn new(args: &[&str]) -> Self {
        let owned: Vec<CString> = args.iter().map(|a| CString::new(*a).unwrap()).collect();
        let mut ptrs: Vec<*mut c_char> = owned.iter().map(|c| c.as_ptr() as *mut c_char).collect();
        ptrs.push(std::ptr::null_mut());
        Argv { _owned: owned, ptrs }
    }
    pub fn argc(&self) -> c_int {
        (self.ptrs.len() - 1) as c_int
    }
    pub fn as_ptr(&self) -> *mut *mut c_char {
        self.ptrs.as_ptr() as *mut *mut c_char
    }
}

// ---------------------------------------------------------------------------
// deterministic RNG (splitmix64) — property-style inputs, reproducible
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next_u64() % n as u64) as usize
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    /// Any byte except NUL (NUL would terminate the C string).
    pub fn byte_nonzero(&mut self) -> u8 {
        (1 + (self.next_u64() % 255)) as u8
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

// ---------------------------------------------------------------------------
// minimal single-threaded test runner (`harness = false`)
// ---------------------------------------------------------------------------

/// Sequential row runner.  Keeps the suite single threaded so that nothing else
/// can write to fd 1 while a capture is in progress.
pub struct Suite {
    name: &'static str,
    filter: Option<String>,
    passed: usize,
    skipped: usize,
    failures: Vec<(String, String)>,
}

impl Suite {
    pub fn new(name: &'static str) -> Self {
        let filter = std::env::args().skip(1).find(|a| !a.starts_with("--"));
        println!("running suite `{name}`");
        Suite {
            name,
            filter,
            passed: 0,
            skipped: 0,
            failures: Vec::new(),
        }
    }

    pub fn run(&mut self, row: &str, f: impl FnOnce()) {
        if let Some(ref pat) = self.filter {
            if !row.contains(pat.as_str()) {
                self.skipped += 1;
                return;
            }
        }
        print!("  {row} ... ");
        let _ = std::io::stdout().flush();
        let res = std::panic::catch_unwind(AssertUnwindSafe(f));
        match res {
            Ok(()) => {
                self.passed += 1;
                println!("ok");
            }
            Err(p) => {
                let msg = if let Some(s) = p.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = p.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "<non-string panic payload>".to_string()
                };
                println!("FAILED");
                self.failures.push((row.to_string(), msg));
            }
        }
        let _ = std::io::stdout().flush();
    }

    pub fn finish(self) {
        println!(
            "\nsuite `{}`: {} passed, {} failed, {} skipped ({} captured .so calls)",
            self.name,
            self.passed,
            self.failures.len(),
            self.skipped,
            CAPTURES.load(Ordering::Relaxed)
        );
        if !self.failures.is_empty() {
            println!("\nfailures:");
            for (row, msg) in self.failures.iter() {
                println!("---- {row} ----\n{msg}\n");
            }
            let _ = std::io::stdout().flush();
            std::process::exit(1);
        }
        let _ = std::io::stdout().flush();
    }
}

/// Sanity helper used by a few rows: what the C `main` prints.
pub const EXPECTED_PROGRAM_OUTPUT: &[u8] =
    b"Calling good()...\n0\n2\nFinished good()\nCalling bad()...\n0\n0\nFinished bad()\n";

pub fn cstr_bytes(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        Vec::new()
    } else {
        unsafe { CStr::from_ptr(p).to_bytes().to_vec() }
    }
}
