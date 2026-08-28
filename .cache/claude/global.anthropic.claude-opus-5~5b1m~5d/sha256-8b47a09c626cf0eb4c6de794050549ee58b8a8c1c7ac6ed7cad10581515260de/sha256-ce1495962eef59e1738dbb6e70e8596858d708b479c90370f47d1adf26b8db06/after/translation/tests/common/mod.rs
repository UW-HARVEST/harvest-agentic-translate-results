// Shared differential-test scaffolding.
//
// Both implementations are loaded as shared objects through `libloading` and
// invoked only through their exported C symbols, so the `#[no_mangle]` wrappers
// are part of what is under test. Nothing in the Rust crate is called directly.
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_void};
use std::ffi::CString;
use std::io::Write;
use std::path::PathBuf;
use std::ptr;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// Minimal sequential test runner (`harness = false`)
//
// All progress/diagnostics go to stderr so that fd 1 — which the differential
// tests hand over to the two shared objects — carries nothing but the bytes the
// libraries themselves write.
// ---------------------------------------------------------------------------

pub fn run_tests(tests: &[(&'static str, fn())]) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let filter = args.iter().find(|a| !a.starts_with('-')).cloned();
    let exact = args.iter().any(|a| a == "--exact");

    let mut failed: Vec<&str> = Vec::new();
    let mut ran = 0usize;
    let mut skipped = 0usize;

    for (name, f) in tests {
        if let Some(fl) = &filter {
            let hit = if exact { *name == fl.as_str() } else { name.contains(fl.as_str()) };
            if !hit {
                skipped += 1;
                continue;
            }
        }
        eprint!("test {name} ... ");
        let _ = std::io::stderr().flush();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        // Make sure fd 1 is back in place before reporting.
        let _ = std::io::stdout().flush();
        match outcome {
            Ok(()) => eprintln!("ok"),
            Err(_) => {
                eprintln!("FAILED");
                failed.push(name);
            }
        }
        ran += 1;
    }

    eprintln!(
        "\ntest result: {}. {} passed; {} failed; {} filtered out",
        if failed.is_empty() { "ok" } else { "FAILED" },
        ran - failed.len(),
        failed.len(),
        skipped
    );
    if !failed.is_empty() {
        eprintln!("failures:");
        for f in &failed {
            eprintln!("    {f}");
        }
        std::process::exit(1);
    }
    if ran == 0 {
        eprintln!("no test matched the filter {filter:?}");
        std::process::exit(2);
    }
}

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn pread(fd: c_int, buf: *mut c_void, n: usize, off: i64) -> isize;
    fn ftruncate(fd: c_int, len: i64) -> c_int;
    fn unlink(path: *const c_char) -> c_int;
    pub fn malloc(n: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn getpid() -> c_int;
}

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;
const SEEK_SET: c_int = 0;
const SEEK_CUR: c_int = 1;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub type CleanupFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type PrintResultFn = unsafe extern "C" fn(*const c_char, c_int);
pub type CleanupResourcesFn = unsafe extern "C" fn(*mut c_char);

pub struct Impl {
    pub name: &'static str,
    pub cleanup: CleanupFn,
    pub print_result: PrintResultFn,
    pub cleanup_resources: CleanupResourcesFn,
    _lib: libloading::Library,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));
        unsafe {
            let cleanup = *lib
                .get::<CleanupFn>(b"cleanup\0")
                .expect("missing symbol `cleanup`");
            let print_result = *lib
                .get::<PrintResultFn>(b"print_result\0")
                .expect("missing symbol `print_result`");
            let cleanup_resources = *lib
                .get::<CleanupResourcesFn>(b"cleanup_resources\0")
                .expect("missing symbol `cleanup_resources`");
            Impl {
                name,
                cleanup,
                print_result,
                cleanup_resources,
                _lib: lib,
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C `.so` produced by `c_src/CMakeLists.txt`. The cmake target name is
/// derived from the parent directory name, so the file name is not fixed.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let dir = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            n.starts_with("lib") && n.ends_with(".so")
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        dir.display(),
        found
    );
    found.pop().unwrap()
}

/// The Rust `cdylib`, taken from the same target profile directory as the
/// running test binary (`target/<profile>/deps/<test>` -> `target/<profile>`).
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let p = profile_dir.join("libcleanup_lib.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {} — run `cargo build` for this profile first",
        p.display()
    );
    p
}

pub fn c() -> &'static Impl {
    static C: OnceLock<Impl> = OnceLock::new();
    C.get_or_init(|| Impl::load("C", &c_so_path()))
}

pub fn rs() -> &'static Impl {
    static R: OnceLock<Impl> = OnceLock::new();
    R.get_or_init(|| Impl::load("Rust", &rust_so_path()))
}

/// Force both libraries to be resolved before any stdout redirection or
/// allocator interposition is armed.
pub fn preload_both() {
    let _ = c();
    let _ = rs();
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

pub fn stdout_guard() -> MutexGuard<'static, ()> {
    STDOUT_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Redirects fd 1 to a scratch file so the exact bytes written by each library
/// can be compared. Both `.so`s share the process' single `stdout` `FILE`, so
/// `fflush(NULL)` before every read is what makes the comparison exact.
pub struct Capture {
    saved: c_int,
    fd: c_int,
    off: i64,
    path: CString,
    _guard: MutexGuard<'static, ()>,
}

impl Capture {
    pub fn new(tag: &str) -> Capture {
        // Resolve libraries (dlopen prints nothing, but keep it out of the
        // captured range regardless).
        preload_both();
        let guard = stdout_guard();
        let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        let path = CString::new(format!(
            "{}/harvest-cap-{}-{}-{:p}.out",
            dir.trim_end_matches('/'),
            unsafe { getpid() },
            tag,
            &guard as *const _
        ))
        .unwrap();
        // Drain Rust's own stdout buffer so nothing of ours lands in the file.
        let _ = std::io::stdout().flush();
        unsafe {
            fflush(ptr::null_mut());
            let fd = open(path.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
            assert!(fd >= 0, "open({:?}) failed", path);
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(fd, 1) >= 0, "dup2 failed");
            Capture {
                saved,
                fd,
                off: 0,
                path,
                _guard: guard,
            }
        }
    }

    /// Bytes written to stdout since the previous `take`.
    pub fn take(&mut self) -> Vec<u8> {
        unsafe {
            fflush(ptr::null_mut());
            let end = lseek(self.fd, 0, SEEK_CUR);
            let len = (end - self.off).max(0) as usize;
            let mut buf = vec![0u8; len];
            if len > 0 {
                let n = pread(self.fd, buf.as_mut_ptr() as *mut c_void, len, self.off);
                assert_eq!(n, len as isize, "short pread while capturing stdout");
            }
            self.off = end;
            if end > (1 << 22) {
                ftruncate(self.fd, 0);
                lseek(self.fd, 0, SEEK_SET);
                self.off = 0;
            }
            buf
        }
    }

    /// Drop pending output without materialising it.
    pub fn discard(&mut self) {
        unsafe {
            fflush(ptr::null_mut());
            ftruncate(self.fd, 0);
            lseek(self.fd, 0, SEEK_SET);
            self.off = 0;
        }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        unsafe {
            fflush(ptr::null_mut());
            dup2(self.saved, 1);
            close(self.saved);
            close(self.fd);
            unlink(self.path.as_ptr());
        }
    }
}

pub fn show(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// One `cleanup(a,b,c,d)` call against both libraries; compares the returned
/// `int` and the exact stdout bytes.
pub fn diff_cleanup(cap: &mut Capture, a: i32, b: i32, cc: i32, d: i32) -> i32 {
    let _ = cap.take(); // start from a clean slate
    let rc_c = unsafe { (c().cleanup)(a, b, cc, d) };
    let out_c = cap.take();
    let rc_r = unsafe { (rs().cleanup)(a, b, cc, d) };
    let out_r = cap.take();
    assert_eq!(
        rc_c, rc_r,
        "cleanup({a},{b},{cc},{d}) return mismatch: C={rc_c} Rust={rc_r}"
    );
    assert_eq!(
        out_c,
        out_r,
        "cleanup({a},{b},{cc},{d}) stdout mismatch:\n  C   = \"{}\"\n  Rust= \"{}\"",
        show(&out_c),
        show(&out_r)
    );
    rc_c
}

/// Same as `diff_cleanup` but also returns the (identical) stdout bytes.
pub fn diff_cleanup_out(cap: &mut Capture, a: i32, b: i32, cc: i32, d: i32) -> (i32, Vec<u8>) {
    let _ = cap.take();
    let rc_c = unsafe { (c().cleanup)(a, b, cc, d) };
    let out_c = cap.take();
    let rc_r = unsafe { (rs().cleanup)(a, b, cc, d) };
    let out_r = cap.take();
    assert_eq!(rc_c, rc_r, "cleanup({a},{b},{cc},{d}) return mismatch");
    assert_eq!(
        out_c,
        out_r,
        "cleanup({a},{b},{cc},{d}) stdout mismatch:\n  C   = \"{}\"\n  Rust= \"{}\"",
        show(&out_c),
        show(&out_r)
    );
    (rc_c, out_c)
}

/// One `print_result(label, n)` call against both libraries. `label` is passed
/// as a raw pointer so `NULL` can be exercised.
pub fn diff_print_result(cap: &mut Capture, label: *const c_char, n: i32, what: &str) -> Vec<u8> {
    let _ = cap.take();
    unsafe { (c().print_result)(label, n) };
    let out_c = cap.take();
    unsafe { (rs().print_result)(label, n) };
    let out_r = cap.take();
    assert_eq!(
        out_c.len(),
        out_r.len(),
        "print_result({what}, {n}) stdout length mismatch: C={} Rust={}",
        out_c.len(),
        out_r.len()
    );
    assert!(
        out_c == out_r,
        "print_result({what}, {n}) stdout mismatch:\n  C   = \"{}\"\n  Rust= \"{}\"",
        show(&out_c[..out_c.len().min(200)]),
        show(&out_r[..out_r.len().min(200)])
    );
    out_c
}

/// `cleanup_resources` on a freshly allocated block of `size` bytes, once
/// through each library. Asserts no output and no crash; a mismatch in which
/// allocator is used would abort the process here.
pub fn diff_cleanup_resources(cap: &mut Capture, size: usize) {
    let _ = cap.take();
    unsafe {
        let p = malloc(size) as *mut c_char;
        assert!(!p.is_null() || size > (1 << 40), "malloc({size}) failed");
        (c().cleanup_resources)(p);
        let out_c = cap.take();

        let q = malloc(size) as *mut c_char;
        assert!(!q.is_null() || size > (1 << 40), "malloc({size}) failed");
        (rs().cleanup_resources)(q);
        let out_r = cap.take();

        assert!(
            out_c.is_empty() && out_r.is_empty(),
            "cleanup_resources({size}) must print nothing: C=\"{}\" Rust=\"{}\"",
            show(&out_c),
            show(&out_r)
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0FF_EE00_1234;

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
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len())]
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

/// Values the `switch` treats specially plus every interesting boundary.
pub const CASE_LABELS: [i32; 4] = [10, 20, 30, 40];
pub const NEAR_CASE: [i32; 8] = [9, 11, 19, 21, 29, 31, 39, 41];
pub const NEGATED_CASE: [i32; 4] = [-10, -20, -30, -40];
pub const EXTREMES: [i32; 6] = [0, 1, -1, i32::MIN, i32::MAX, i32::MIN + 1];

/// Reference model of the C `switch` (with its fall-through), used only to
/// cross-check that the differential tests actually reach the interesting arms.
pub fn model_cleanup(a: i32, b: i32, c_: i32, d: i32) -> i32 {
    let mut r: i32 = 0;
    for n in [a, b, c_, d] {
        r = match n {
            10 => r.wrapping_add(10).wrapping_add(20), // falls through into case 20
            20 => r.wrapping_add(20),
            30 => r.wrapping_add(30).wrapping_add(40), // falls through into case 40
            40 => r.wrapping_add(40),
            other => r.wrapping_add(other),
        };
    }
    r
}

pub const EXPECTED_CLEANUP_STDOUT: &[u8] = b"Processed numbers: numbers\n";
