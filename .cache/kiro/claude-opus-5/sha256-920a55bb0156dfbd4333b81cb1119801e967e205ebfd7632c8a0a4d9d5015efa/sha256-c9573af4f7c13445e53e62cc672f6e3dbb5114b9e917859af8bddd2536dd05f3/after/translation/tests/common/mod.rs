//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects with `libloading` and driven
//! **only** through their exported symbols — the Rust crate is never linked
//! directly, so the `#[unsafe(no_mangle)] extern "C"` wrappers are part of what
//! is under test.

#![allow(dead_code)]

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::Library;

/// `typedef struct { int** matrix; int width; int height; } matrix_t;`
#[repr(C)]
pub struct MatrixT {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

pub type FnAllocateMatrix = unsafe extern "C" fn(c_int, c_int) -> *mut MatrixT;
pub type FnFreeMatrix = unsafe extern "C" fn(*mut MatrixT);
pub type FnInitFromString = unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut MatrixT;
pub type FnMultiply = unsafe extern "C" fn(*mut MatrixT, *mut MatrixT) -> *mut MatrixT;
pub type FnMatrixToString = unsafe extern "C" fn(*mut MatrixT) -> *mut c_char;
pub type FnWriteToFile = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
pub type FnDriver =
    unsafe extern "C" fn(c_int, c_int, *const c_char, c_int, c_int, *const c_char) -> c_int;

unsafe extern "C" {
    /// The process-wide glibc `free`; both `.so`s allocate with the same
    /// `malloc`, so the test may release their return values.
    pub fn free(ptr: *mut c_void);
    pub fn strlen(s: *const c_char) -> usize;
}

pub struct Api {
    pub name: &'static str,
    pub allocate_matrix: FnAllocateMatrix,
    pub free_matrix: FnFreeMatrix,
    pub initialize_matrix_from_string: FnInitFromString,
    pub multiply_matrices: FnMultiply,
    pub matrix_to_string: FnMatrixToString,
    pub write_to_file: FnWriteToFile,
    pub driver: FnDriver,
    _lib: Library,
}

impl Api {
    fn load(name: &'static str, path: &PathBuf) -> Api {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));
        macro_rules! sym {
            ($t:ty, $s:expr) => {{
                let name: &str = $s;
                let nul = format!("{name}\0");
                unsafe {
                    *lib.get::<$t>(nul.as_bytes())
                        .unwrap_or_else(|e| panic!("missing symbol {name} in {}: {e}", path.display()))
                }
            }};
        }
        let api = Api {
            name,
            allocate_matrix: sym!(FnAllocateMatrix, "allocate_matrix"),
            free_matrix: sym!(FnFreeMatrix, "free_matrix"),
            initialize_matrix_from_string: sym!(FnInitFromString, "initialize_matrix_from_string"),
            multiply_matrices: sym!(FnMultiply, "multiply_matrices"),
            matrix_to_string: sym!(FnMatrixToString, "matrix_to_string"),
            write_to_file: sym!(FnWriteToFile, "write_to_file"),
            driver: sym!(FnDriver, "driver"),
            _lib: lib,
        };
        api
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("crate has a parent directory")
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // target/<profile>/deps/<test-exe>  ->  target/<profile>/libdriver.so
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("test exe lives in target/<profile>/deps");
    let candidate = profile_dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    for p in ["release", "debug"] {
        let c = manifest_dir().join("target").join(p).join("libdriver.so");
        if c.exists() {
            return c;
        }
    }
    panic!("could not locate the Rust libdriver.so (set DRIVER_RUST_SO)");
}

static C_API: OnceLock<&'static Api> = OnceLock::new();
static RUST_API: OnceLock<&'static Api> = OnceLock::new();

pub fn c_api() -> &'static Api {
    C_API.get_or_init(|| Box::leak(Box::new(Api::load("C", &c_so_path()))))
}

pub fn rust_api() -> &'static Api {
    RUST_API.get_or_init(|| Box::leak(Box::new(Api::load("Rust", &rust_so_path()))))
}

/// `[c_api(), rust_api()]` — iterate to run the identical sequence on both.
pub fn both() -> [&'static Api; 2] {
    [c_api(), rust_api()]
}

// ---------------------------------------------------------------------------
// matrix helpers
// ---------------------------------------------------------------------------

/// Owned snapshot of a `matrix_t` as seen through the exported struct layout.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Snapshot {
    pub width: c_int,
    pub height: c_int,
    pub rows: Vec<Vec<c_int>>,
}

/// Reads a `matrix_t*` (which must be non-NULL) into an owned snapshot.
///
/// # Safety
/// `mat` must point at a live matrix whose `height` rows each hold `width`
/// initialised `int`s.
pub unsafe fn snapshot(mat: *mut MatrixT) -> Snapshot {
    unsafe {
        let width = (*mat).width;
        let height = (*mat).height;
        let mut rows = Vec::new();
        for i in 0..height.max(0) {
            let row = *(*mat).matrix.offset(i as isize);
            let mut vals = Vec::new();
            for j in 0..width.max(0) {
                vals.push(*row.offset(j as isize));
            }
            rows.push(vals);
        }
        Snapshot {
            width,
            height,
            rows,
        }
    }
}

/// Reads the row pointers of a matrix (used to assert they are non-NULL).
///
/// # Safety
/// `mat` must be a live matrix with a valid `height`.
pub unsafe fn row_ptrs(mat: *mut MatrixT) -> Vec<*mut c_int> {
    unsafe {
        (0..(*mat).height.max(0))
            .map(|i| *(*mat).matrix.offset(i as isize))
            .collect()
    }
}

/// Takes ownership of a `char*` returned by either library, returning its bytes
/// (excluding the NUL) and `free`ing it.
///
/// # Safety
/// `p` must be NULL or a `malloc`ed NUL-terminated string.
pub unsafe fn take_c_string(p: *mut c_char) -> Option<Vec<u8>> {
    unsafe {
        if p.is_null() {
            return None;
        }
        let bytes = CStr::from_ptr(p).to_bytes().to_vec();
        free(p as *mut c_void);
        Some(bytes)
    }
}

pub fn cstring(s: &str) -> CString {
    CString::new(s).expect("test input contains an interior NUL")
}

/// Builds a matrix through `allocate_matrix` and fills it with `rows`.
///
/// # Safety
/// Caller must eventually pass the result to `free_matrix` of the same library.
pub unsafe fn build_matrix(api: &Api, rows: &[Vec<c_int>], width: c_int) -> *mut MatrixT {
    unsafe {
        let height = rows.len() as c_int;
        let mat = (api.allocate_matrix)(width, height);
        assert!(!mat.is_null(), "{}: allocate_matrix failed", api.name);
        for (i, r) in rows.iter().enumerate() {
            assert_eq!(r.len() as c_int, width);
            let row = *(*mat).matrix.add(i);
            for (j, v) in r.iter().enumerate() {
                *row.add(j) = *v;
            }
        }
        mat
    }
}

// ---------------------------------------------------------------------------
// deterministic PRNG (splitmix64) — fixed seed, reproducible
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

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
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    pub fn i32_in(&mut self, lo: i32, hi: i32) -> i32 {
        self.range(lo as i64, hi as i64) as i32
    }
    pub fn any_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

/// Renders `rows` in the canonical form the C parser expects.
pub fn canonical(rows: &[Vec<c_int>]) -> String {
    let mut s = String::new();
    for r in rows {
        let cells: Vec<String> = r.iter().map(|v| v.to_string()).collect();
        s.push_str(&cells.join(" "));
        s.push('\n');
    }
    s
}

/// A unique scratch directory for a test (created on demand).
pub fn scratch_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("driver_diff_{}_{}", std::process::id(), tag));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("create scratch dir");
    d
}
