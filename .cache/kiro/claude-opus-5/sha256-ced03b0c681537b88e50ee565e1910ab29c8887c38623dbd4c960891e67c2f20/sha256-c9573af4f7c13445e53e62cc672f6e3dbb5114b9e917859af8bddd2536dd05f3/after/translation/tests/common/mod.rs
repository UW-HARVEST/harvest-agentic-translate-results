//! Shared differential-test harness.
//!
//! Loads BOTH shared objects with `libloading` — the CMake-built C
//! `libdriver.so` and the Rust `cdylib` `libdriver.so` — and calls every
//! function through its exported symbol, exactly as an external C consumer
//! would. No Rust function is ever called directly, so the `#[no_mangle]`
//! wrappers are part of what is under test.

#![allow(dead_code)]

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};

/// Mirror of `typedef struct { int** matrix; int width; int height; } matrix_t;`
#[repr(C)]
#[derive(Debug)]
pub struct MatrixT {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

pub type FnAllocate = unsafe extern "C" fn(c_int, c_int) -> *mut MatrixT;
pub type FnFreeMatrix = unsafe extern "C" fn(*mut MatrixT);
pub type FnInit = unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut MatrixT;
pub type FnMultiply = unsafe extern "C" fn(*mut MatrixT, *mut MatrixT) -> *mut MatrixT;
pub type FnToString = unsafe extern "C" fn(*mut MatrixT) -> *mut c_char;
pub type FnWrite = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
pub type FnDriver =
    unsafe extern "C" fn(c_int, c_int, *const c_char, c_int, c_int, *const c_char) -> c_int;

unsafe extern "C" {
    fn free(p: *mut c_void);
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    static mut stderr: *mut c_void;
}

/// libc `free`, for releasing pointers either `.so` returned.
pub unsafe fn libc_free(p: *mut c_void) {
    unsafe { free(p) }
}

/// The seven exported entry points of one implementation.
pub struct Api {
    pub name: &'static str,
    pub allocate_matrix: FnAllocate,
    pub free_matrix: FnFreeMatrix,
    pub initialize_matrix_from_string: FnInit,
    pub multiply_matrices: FnMultiply,
    pub matrix_to_string: FnToString,
    pub write_to_file: FnWrite,
    pub driver: FnDriver,
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = crate_root().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with cmake first"
    );
    p
}

fn rust_so_path() -> PathBuf {
    // Walk up from the test executable (target/<profile>/deps/<name>-<hash>).
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir: Option<&Path> = exe.parent();
    while let Some(d) = dir {
        let cand = d.join("libdriver.so");
        if cand.exists() {
            return cand;
        }
        dir = d.parent();
    }
    // Fall back to the usual locations.
    for p in ["target/release/libdriver.so", "target/debug/libdriver.so"] {
        let cand = crate_root().join(p);
        if cand.exists() {
            return cand;
        }
    }
    panic!("Rust cdylib libdriver.so not found; run `cargo build --release` first");
}

unsafe fn load_api(path: &Path, name: &'static str) -> Api {
    unsafe {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("failed to dlopen {path:?}: {e}"));
        // Keep the library resident for the whole process so the extracted raw
        // function pointers stay valid.
        let lib: &'static libloading::Library = Box::leak(Box::new(lib));
        macro_rules! sym {
            ($s:literal, $t:ty) => {{
                let s: libloading::Symbol<$t> = lib
                    .get(concat!($s, "\0").as_bytes())
                    .unwrap_or_else(|e| panic!("{} missing symbol {}: {}", name, $s, e));
                *s
            }};
        }
        Api {
            name,
            allocate_matrix: sym!("allocate_matrix", FnAllocate),
            free_matrix: sym!("free_matrix", FnFreeMatrix),
            initialize_matrix_from_string: sym!("initialize_matrix_from_string", FnInit),
            multiply_matrices: sym!("multiply_matrices", FnMultiply),
            matrix_to_string: sym!("matrix_to_string", FnToString),
            write_to_file: sym!("write_to_file", FnWrite),
            driver: sym!("driver", FnDriver),
        }
    }
}

/// Both implementations, ready to be driven side by side.
pub struct Both {
    pub c: Api,
    pub rs: Api,
}

pub fn load_both() -> Both {
    unsafe {
        Both {
            c: load_api(&c_so_path(), "C"),
            rs: load_api(&rust_so_path(), "Rust"),
        }
    }
}

// ---------------------------------------------------------------- matrix I/O

/// Snapshot of a `matrix_t` as seen from outside the library.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Snapshot {
    pub width: c_int,
    pub height: c_int,
    /// `None` when the row pointer was NULL.
    pub rows: Vec<Option<Vec<c_int>>>,
}

/// Reads a `matrix_t*` produced by either `.so`. Row contents are only read
/// when `width > 0 && height > 0`, matching what the C itself would touch.
pub unsafe fn snapshot(mat: *mut MatrixT) -> Option<Snapshot> {
    unsafe {
        if mat.is_null() {
            return None;
        }
        let width = (*mat).width;
        let height = (*mat).height;
        let mut rows = Vec::new();
        if height > 0 {
            for i in 0..height {
                let row = *(*mat).matrix.offset(i as isize);
                if row.is_null() {
                    rows.push(None);
                } else if width > 0 {
                    let mut v = Vec::with_capacity(width as usize);
                    for j in 0..width {
                        v.push(*row.offset(j as isize));
                    }
                    rows.push(Some(v));
                } else {
                    rows.push(Some(Vec::new()));
                }
            }
        }
        Some(Snapshot {
            width,
            height,
            rows,
        })
    }
}

/// Allocates a matrix through `api.allocate_matrix` and fills it with `values`
/// (row-major, `height * width` entries).
pub unsafe fn make_matrix(api: &Api, width: c_int, height: c_int, values: &[c_int]) -> *mut MatrixT {
    unsafe {
        assert_eq!(values.len(), (width.max(0) as usize) * (height.max(0) as usize));
        let mat = (api.allocate_matrix)(width, height);
        assert!(!mat.is_null(), "{} allocate_matrix({width},{height}) failed", api.name);
        for i in 0..height {
            let row = *(*mat).matrix.offset(i as isize);
            for j in 0..width {
                *row.offset(j as isize) = values[(i as usize) * (width as usize) + j as usize];
            }
        }
        mat
    }
}

/// Copies a NUL-terminated string out of either `.so`'s heap.
pub unsafe fn cstr_bytes(p: *const c_char) -> Option<Vec<u8>> {
    unsafe {
        if p.is_null() {
            return None;
        }
        let mut out = Vec::new();
        let mut q = p;
        while *q != 0 {
            out.push(*q as u8);
            q = q.add(1);
        }
        Some(out)
    }
}

// ------------------------------------------------------------ stderr capture

/// Runs `f` with fd 2 redirected to a temp file and returns the bytes written.
///
/// Both `.so`s use the process-wide glibc `stderr`, so this captures
/// `perror`/`fprintf(stderr, …)` diagnostics from either implementation.
/// fd 2 is process-wide, so the redirection is serialised behind a mutex.
pub fn capture_stderr<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = std::env::temp_dir().join(format!(
        "difftest-stderr-{}-{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_file(&path);
    let cpath = CString::new(path.to_str().unwrap()).unwrap();
    const O_WRONLY: c_int = 1;
    const O_CREAT: c_int = 0o100;
    const O_TRUNC: c_int = 0o1000;
    unsafe {
        fflush(stderr);
        let saved = dup(2);
        assert!(saved >= 0, "dup(2) failed");
        let fd = open(cpath.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o644 as c_int);
        assert!(fd >= 0, "open temp stderr file failed");
        dup2(fd, 2);
        close(fd);
        let r = f();
        fflush(stderr);
        dup2(saved, 2);
        close(saved);
        let bytes = std::fs::read(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        (r, bytes)
    }
}

// -------------------------------------------------------------------- random

/// Deterministic xorshift64* PRNG so every randomized row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next_u64() as usize) % xs.len()]
    }
    /// A value whose decimal form is at most 10 characters, which is the range
    /// `matrix_to_string`'s buffer formula provably accommodates.
    pub fn safe_value(&mut self) -> c_int {
        self.range(-999_999_999, 999_999_999) as c_int
    }
}

/// Renders a matrix as the whitespace/newline text form the parser accepts.
pub fn render_matrix_text(width: usize, height: usize, values: &[c_int]) -> String {
    let mut s = String::new();
    for i in 0..height {
        for j in 0..width {
            if j > 0 {
                s.push(' ');
            }
            s.push_str(&values[i * width + j].to_string());
        }
        s.push('\n');
    }
    s
}

pub fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}
