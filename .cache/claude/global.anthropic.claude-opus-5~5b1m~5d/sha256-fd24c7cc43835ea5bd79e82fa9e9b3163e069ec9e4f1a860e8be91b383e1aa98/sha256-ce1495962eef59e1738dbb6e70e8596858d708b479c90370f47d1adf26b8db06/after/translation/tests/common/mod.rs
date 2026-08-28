//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading`; the Rust
//! functions are NEVER called directly, always through `dlsym` on the built
//! cdylib, exactly as an external C consumer would. That way the
//! `#[no_mangle] extern "C"` export wrappers are part of what is under test.

#![allow(dead_code)]

use libloading::os::unix::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// FFI signatures (mirroring c_src/src/lib.c exactly)
// ---------------------------------------------------------------------------

pub type FnCreateResultString = unsafe extern "C" fn(*const c_char, c_int) -> *mut c_char;
pub type FnCheckPermissions = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnSafeAdd = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type FnMultiplyWithLog = unsafe extern "C" fn(c_int, c_int, *mut *mut c_char) -> c_int;
pub type FnCopyAndSum = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
pub type FnCompareOperations = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
pub type FnComplexmode = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded shared object with all seven exports resolved eagerly.
pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    pub create_result_string: FnCreateResultString,
    pub check_permissions: FnCheckPermissions,
    pub safe_add: FnSafeAdd,
    pub multiply_with_log: FnMultiplyWithLog,
    pub copy_and_sum: FnCopyAndSum,
    pub compare_operations: FnCompareOperations,
    pub complexmode: FnComplexmode,
    // Dropped last; keeps the mapping alive for the whole process.
    _lib: Library,
}

unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

impl Lib {
    pub fn open(name: &'static str, path: &Path) -> Lib {
        // RTLD_NOW so that every PLT entry is bound up-front. The OOM tests
        // deliberately exhaust the heap, and lazy binding would otherwise need
        // to allocate inside the dynamic loader at call time.
        let lib = unsafe {
            Library::open(Some(path), libc::RTLD_NOW | libc::RTLD_LOCAL)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
        };
        unsafe fn sym<T>(lib: &Library, n: &[u8]) -> T {
            let s: Symbol<T> = unsafe {
                lib.get(n).unwrap_or_else(|e| {
                    panic!("dlsym({}) failed: {e}", String::from_utf8_lossy(n))
                })
            };
            unsafe { std::ptr::read(&*s as *const T) }
        }
        unsafe {
            Lib {
                name,
                path: path.to_path_buf(),
                create_result_string: sym(&lib, b"create_result_string\0"),
                check_permissions: sym(&lib, b"check_permissions\0"),
                safe_add: sym(&lib, b"safe_add\0"),
                multiply_with_log: sym(&lib, b"multiply_with_log\0"),
                copy_and_sum: sym(&lib, b"copy_and_sum\0"),
                compare_operations: sym(&lib, b"compare_operations\0"),
                complexmode: sym(&lib, b"complexmode\0"),
                _lib: lib,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<project>.so` — the project name is derived from the parent
/// directory name by `c_src/CMakeLists.txt`, so the file is located by scanning
/// rather than by hard-coding a name.
pub fn c_so_path() -> PathBuf {
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// `libcomplexmode_lib.so` from the SAME profile directory as this test binary.
///
/// The test executable lives at `target/<profile>/deps/<name>-<hash>`, so the
/// cdylib sits in `../`. Deriving the path this way (instead of preferring
/// `release/` or picking the newest file) guarantees that `cargo test` and
/// `cargo test --release` each exercise the cdylib built with the very same
/// flags — which matters, because `debug_assertions` changes the generated code
/// (e.g. raw-pointer null-check instrumentation).
pub fn rust_so_path() -> PathBuf {
    const NAME: &str = "libcomplexmode_lib.so";
    let exe = std::env::current_exe().expect("current_exe");
    if let Some(profile_dir) = exe.parent().and_then(|p| p.parent()) {
        let p = profile_dir.join(NAME);
        if p.is_file() {
            return p;
        }
    }
    // Fallback for unusual layouts.
    let base = manifest_dir().join("target");
    for sub in ["release", "debug"] {
        let p = base.join(sub).join(NAME);
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "no {NAME} next to the test binary ({}) nor under {}; run `cargo build` first",
        exe.display(),
        base.display()
    )
}

static LIBS: OnceLock<(Lib, Lib)> = OnceLock::new();

/// `(c, rust)` — loaded once per test process.
pub fn libs() -> &'static (Lib, Lib) {
    LIBS.get_or_init(|| {
        (
            Lib::open("C", &c_so_path()),
            Lib::open("Rust", &rust_so_path()),
        )
    })
}

// ---------------------------------------------------------------------------
// stdout capture
//
// Both `.so`s reach stdout through the same libc `FILE*`, so the only reliable
// way to compare their printed bytes is to redirect fd 1 around each call.
// A process-wide mutex serialises this (test binaries are separate processes,
// so there is no cross-binary interference).
// ---------------------------------------------------------------------------

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        // Flush anything the harness left pending so it is not attributed to
        // the call under test.
        libc::fflush(std::ptr::null_mut());

        let mut tmpl: Vec<u8> = std::env::temp_dir()
            .join("difftest_stdout_XXXXXX")
            .as_os_str()
            .as_encoded_bytes()
            .to_vec();
        tmpl.push(0);
        let tmp_fd = libc::mkstemp(tmpl.as_mut_ptr() as *mut c_char);
        assert!(tmp_fd >= 0, "mkstemp failed");
        let path = CStr::from_ptr(tmpl.as_ptr() as *const c_char).to_owned();

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(tmp_fd, 1) >= 0, "dup2 failed");

        let out = f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);
        libc::lseek(tmp_fd, 0, libc::SEEK_SET);

        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let n = libc::read(tmp_fd, chunk.as_mut_ptr() as *mut c_void, chunk.len());
            if n <= 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n as usize]);
        }
        libc::close(tmp_fd);
        libc::unlink(path.as_ptr());
        (out, buf)
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep every row reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Full-range `i32`, including `INT_MIN` / `INT_MAX`.
    pub fn i32_any(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Small magnitude value, so products/sums usually do NOT overflow.
    pub fn i32_small(&mut self) -> i32 {
        (self.next_u32() % 20001) as i32 - 10000
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    /// Random NUL-free byte string of length `len` (bytes span 0x01..=0xFF, so
    /// the >= 0x80 `strcmp` signedness cases are covered).
    /// Random NUL-free byte string whose length is drawn from `0..max`.
    pub fn cbytes_upto(&mut self, max: u64) -> Vec<u8> {
        let n = self.below(max) as usize;
        self.cbytes(n)
    }
    pub fn cbytes(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| {
                let b = self.byte();
                if b == 0 { 1 } else { b }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Differential assertion helpers
// ---------------------------------------------------------------------------

pub fn cstring(bytes: &[u8]) -> CString {
    CString::new(bytes).expect("interior NUL")
}

/// Read a NUL-terminated buffer produced by either library.
pub unsafe fn read_cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(p) }.to_bytes().to_vec())
    }
}

/// Both libraries allocate with the *same* libc, so either result can be freed
/// here.
pub unsafe fn cfree(p: *mut c_char) {
    if !p.is_null() {
        unsafe { libc::free(p as *mut c_void) }
    }
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).replace('\n', "\\n")
}

/// Compare a `(return value, stdout)` pair from both libraries.
#[track_caller]
pub fn assert_same<T: PartialEq + std::fmt::Debug>(
    ctx: &str,
    c: (T, Vec<u8>),
    r: (T, Vec<u8>),
) {
    assert_eq!(
        c.0, r.0,
        "return value mismatch [{ctx}]: C={:?} Rust={:?}\n  C stdout: {}\n  R stdout: {}",
        c.0,
        r.0,
        show(&c.1),
        show(&r.1)
    );
    assert_eq!(
        c.1,
        r.1,
        "stdout mismatch [{ctx}]:\n  C   : {}\n  Rust: {}",
        show(&c.1),
        show(&r.1)
    );
}
