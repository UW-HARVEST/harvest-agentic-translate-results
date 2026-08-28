//! Shared harness: loads the C reference `.so` and the Rust `cdylib` and
//! exposes both through identical `libloading`-resolved function pointers.
//!
//! Nothing in here calls a Rust function directly; every Rust invocation goes
//! through the dynamic library's exported symbol, exactly like an external
//! C caller, so the `#[no_mangle]` wrappers are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int};
use std::fs;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Library discovery
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ always has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(&build_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_so = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false);
            if is_so {
                candidates.push(path);
            }
        }
    }
    candidates.sort();
    candidates.pop().unwrap_or_else(|| {
        panic!(
            "no C shared library found in {}; build it with \
             `cd c_src && mkdir -p build && cd build && cmake .. \
             -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`",
            build_dir.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // Explicit override, used to verify the release artifact as well.
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "RUST_SO_PATH={} is not a file", p.display());
        return p;
    }
    // The integration-test binary lives in target/<profile>/deps/, so the
    // cdylib produced by the same `cargo test` invocation sits one level up.
    let exe = std::env::current_exe().expect("test executable path");
    let mut dirs = Vec::new();
    if let Some(deps) = exe.parent() {
        if let Some(profile_dir) = deps.parent() {
            dirs.push(profile_dir.to_path_buf());
        }
        dirs.push(deps.to_path_buf());
    }
    let target = workspace_root().join("translation").join("target");
    dirs.push(target.join("debug"));
    dirs.push(target.join("release"));

    for dir in &dirs {
        let candidate = dir.join("libdoubleneg_lib.so");
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!(
        "libdoubleneg_lib.so not found; looked in {:?}. Build it with \
         `cd translation && cargo build`",
        dirs
    );
}

// ---------------------------------------------------------------------------
// Bound symbol table
// ---------------------------------------------------------------------------

type FnConvertDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
type FnFindValueInBuffer = unsafe extern "C" fn(*const c_char, usize, c_int) -> c_int;
type FnProcessNegation = unsafe extern "C" fn(c_int) -> c_int;
type FnCreateNumericBuffer = unsafe extern "C" fn(*mut c_char, c_int, c_int);
type FnCalculateWithDoubles = unsafe extern "C" fn(c_int, c_int, c_int) -> c_double;
type FnDoubleneg = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    convert_double_to_int: FnConvertDoubleToInt,
    find_value_in_buffer: FnFindValueInBuffer,
    process_negation: FnProcessNegation,
    create_numeric_buffer: FnCreateNumericBuffer,
    calculate_with_doubles: FnCalculateWithDoubles,
    doubleneg: FnDoubleneg,
}

impl Impl {
    fn load(name: &'static str, path: PathBuf) -> Impl {
        unsafe {
            let lib = Library::new(&path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));

            macro_rules! sym {
                ($t:ty, $n:literal) => {{
                    let s: Symbol<$t> = lib
                        .get(concat!($n, "\0").as_bytes())
                        .unwrap_or_else(|e| panic!("{} missing symbol {}: {e}", name, $n));
                    *s
                }};
            }

            let convert_double_to_int = sym!(FnConvertDoubleToInt, "convert_double_to_int");
            let find_value_in_buffer = sym!(FnFindValueInBuffer, "find_value_in_buffer");
            let process_negation = sym!(FnProcessNegation, "process_negation");
            let create_numeric_buffer = sym!(FnCreateNumericBuffer, "create_numeric_buffer");
            let calculate_with_doubles = sym!(FnCalculateWithDoubles, "calculate_with_doubles");
            let doubleneg = sym!(FnDoubleneg, "doubleneg");

            Impl {
                name,
                path,
                _lib: lib,
                convert_double_to_int,
                find_value_in_buffer,
                process_negation,
                create_numeric_buffer,
                calculate_with_doubles,
                doubleneg,
            }
        }
    }

    pub fn convert_double_to_int(&self, v: f64) -> i32 {
        unsafe { (self.convert_double_to_int)(v) }
    }

    pub fn find_value_in_buffer(&self, buffer: &[u8], search_val: i32) -> i32 {
        unsafe {
            (self.find_value_in_buffer)(
                buffer.as_ptr().cast::<c_char>(),
                buffer.len(),
                search_val,
            )
        }
    }

    pub fn process_negation(&self, v: i32) -> i32 {
        unsafe { (self.process_negation)(v) }
    }

    /// `buffer` is written in place; `size` is passed through verbatim so that
    /// non-positive sizes can be exercised as well.
    pub fn create_numeric_buffer(&self, buffer: &mut [u8], size: i32, seed: i32) {
        unsafe { (self.create_numeric_buffer)(buffer.as_mut_ptr().cast::<c_char>(), size, seed) }
    }

    pub fn calculate_with_doubles(&self, a: i32, b: i32, c: i32) -> f64 {
        unsafe { (self.calculate_with_doubles)(a, b, c) }
    }

    /// Runs `doubleneg` with fd 1 redirected to a temporary file and returns
    /// `(return value, captured stdout bytes)`.
    pub fn doubleneg_capture(&self, a: i32, b: i32, c: i32, d: i32) -> (i32, Vec<u8>) {
        let capture = StdoutCapture::begin();
        let rv = unsafe { (self.doubleneg)(a, b, c, d) };
        let bytes = capture.finish();
        (rv, bytes)
    }
}

pub fn c_impl() -> &'static Impl {
    static C: OnceLock<Impl> = OnceLock::new();
    C.get_or_init(|| Impl::load("C", find_c_so()))
}

pub fn rust_impl() -> &'static Impl {
    static R: OnceLock<Impl> = OnceLock::new();
    R.get_or_init(|| Impl::load("Rust", find_rust_so()))
}

/// Both implementations, in a fixed order, so callers can loop over them.
pub fn both() -> (&'static Impl, &'static Impl) {
    (c_impl(), rust_impl())
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// Flushes the C library's `stdout`. The C `.so` and this test binary share
    /// the same `libc.so.6`, hence the same `FILE *stdout` buffer.
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

pub struct StdoutCapture {
    saved_fd: c_int,
    file: fs::File,
    path: PathBuf,
}

impl StdoutCapture {
    pub fn begin() -> StdoutCapture {
        // Serialise: fd 1 is process-global.
        let path = std::env::temp_dir().join(format!(
            "doubleneg-stdout-{}-{:?}.bin",
            std::process::id(),
            std::thread::current().id()
        ));
        let file = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .expect("open capture file");

        unsafe {
            fflush(std::ptr::null_mut());
        }
        let saved_fd = unsafe { dup(1) };
        assert!(saved_fd >= 0, "dup(1) failed");
        assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

        StdoutCapture {
            saved_fd,
            file,
            path,
        }
    }

    pub fn finish(self) -> Vec<u8> {
        unsafe {
            // Flush the C stdio buffer before fd 1 is pointed back.
            fflush(std::ptr::null_mut());
            assert!(dup2(self.saved_fd, 1) >= 0, "dup2 restore failed");
            close(self.saved_fd);
        }
        drop(self.file);
        let bytes = fs::read(&self.path).expect("read capture file");
        let _ = fs::remove_file(&self.path);
        bytes
    }
}

/// Guard serialising every stdout-capturing test (fd 1 is process-wide).
pub fn stdout_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

/// Bit-exact float comparison (so NaN payloads and signed zeroes must match).
pub fn assert_f64_bits_eq(c: f64, rust: f64, ctx: &str) {
    assert_eq!(
        c.to_bits(),
        rust.to_bits(),
        "{ctx}: C returned {c:?} (bits {:#018x}), Rust returned {rust:?} (bits {:#018x})",
        c.to_bits(),
        rust.to_bits()
    );
}

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}
