//! Shared harness: loads the C and Rust shared libraries side by side and
//! exposes their exported symbols through identical wrappers.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

/// `matrix_t` from `c_src/include/matrix.h`.
#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

unsafe extern "C" {
    fn free(p: *mut u8);
}

pub type FnAllocateMatrix = unsafe extern "C" fn(c_int, c_int) -> *mut matrix_t;
pub type FnFreeMatrix = unsafe extern "C" fn(*mut matrix_t);
pub type FnInitFromString = unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut matrix_t;
pub type FnMultiply = unsafe extern "C" fn(*mut matrix_t, *mut matrix_t) -> *mut matrix_t;
pub type FnMatrixToString = unsafe extern "C" fn(*mut matrix_t) -> *mut c_char;
pub type FnWriteToFile = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
pub type FnDriver =
    unsafe extern "C" fn(c_int, c_int, *const c_char, c_int, c_int, *const c_char) -> c_int;

/// One loaded implementation (either the C or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub allocate_matrix: FnAllocateMatrix,
    pub free_matrix: FnFreeMatrix,
    pub initialize_matrix_from_string: FnInitFromString,
    pub multiply_matrices: FnMultiply,
    pub matrix_to_string: FnMatrixToString,
    pub write_to_file: FnWriteToFile,
    pub driver: FnDriver,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));

            macro_rules! sym {
                ($t:ty, $s:literal) => {{
                    let s: Symbol<$t> = lib
                        .get($s)
                        .unwrap_or_else(|e| {
                            panic!("{name}: missing symbol {:?}: {e}", $s)
                        });
                    *s
                }};
            }

            let allocate_matrix = sym!(FnAllocateMatrix, b"allocate_matrix\0");
            let free_matrix = sym!(FnFreeMatrix, b"free_matrix\0");
            let initialize_matrix_from_string =
                sym!(FnInitFromString, b"initialize_matrix_from_string\0");
            let multiply_matrices = sym!(FnMultiply, b"multiply_matrices\0");
            let matrix_to_string = sym!(FnMatrixToString, b"matrix_to_string\0");
            let write_to_file = sym!(FnWriteToFile, b"write_to_file\0");
            let driver = sym!(FnDriver, b"driver\0");

            Impl {
                name,
                _lib: lib,
                allocate_matrix,
                free_matrix,
                initialize_matrix_from_string,
                multiply_matrices,
                matrix_to_string,
                write_to_file,
                driver,
            }
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

fn workspace_root() -> PathBuf {
    // tests/ lives in translation/, whose parent is the working directory.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_so() -> PathBuf {
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C library not built at {}; run the cmake build first",
        p.display()
    );
    p
}

/// `cargo test` does not necessarily re-emit the cdylib (no test target links
/// it), so the `.so` on disk can lag behind `src/`. Testing a stale library
/// silently passes regardless of the current source, so refuse to run.
fn assert_fresh(so: &Path) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let mut stack = vec![manifest.join("src")];
    let mut files = vec![manifest.join("Cargo.toml")];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                files.push(p);
            }
        }
    }
    for f in files {
        if let Ok(t) = std::fs::metadata(&f).and_then(|m| m.modified()) {
            if newest.as_ref().is_none_or(|(bt, _)| t > *bt) {
                newest = Some((t, f));
            }
        }
    }
    if let Some((t, f)) = newest {
        assert!(
            t <= so_mtime,
            "{} is older than {}; run `cargo build` (or scripts/verify_all.sh) so the \
             tests exercise the current source",
            so.display(),
            f.display()
        );
    }
}

fn rust_so() -> PathBuf {
    // An explicit override lets the verification script point the tests at a
    // specific build of the cdylib (debug vs release).
    if let Some(p) = std::env::var_os("RUST_DRIVER_SO") {
        let p = PathBuf::from(p);
        assert!(
            p.exists(),
            "RUST_DRIVER_SO points at a missing file: {}",
            p.display()
        );
        assert_fresh(&p);
        return p;
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // cargo builds the cdylib for whichever profile the tests run under.
    for profile in ["debug", "release"] {
        let p = manifest.join("target").join(profile).join("libdriver.so");
        if p.exists() {
            assert_fresh(&p);
            return p;
        }
    }
    panic!("Rust cdylib not found under translation/target/{{debug,release}}");
}

/// Both libraries, loaded once per test process.
pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair {
        c: Impl::load("c", &c_so()),
        rs: Impl::load("rust", &rust_so()),
    })
}

/// Serialises tests that touch the process-wide cwd / `matrix.txt`.
pub fn fs_lock() -> MutexGuard<'static, ()> {
    static L: Mutex<()> = Mutex::new(());
    L.lock().unwrap_or_else(|e| e.into_inner())
}

/// A NUL-terminated owned C string (allows interior bytes that `CString` rejects only for NUL).
pub fn cstr(s: &str) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
    v.push(0);
    v
}

/// Snapshot of a `matrix_t` as observed through the FFI boundary.
#[derive(Debug, PartialEq, Eq)]
pub struct MatSnapshot {
    pub width: c_int,
    pub height: c_int,
    pub rows_null: Vec<bool>,
    pub cells: Vec<Vec<c_int>>,
}

/// Reads a matrix's observable state. `read_cells` controls whether the
/// (possibly uninitialised) cell values are inspected.
pub unsafe fn snapshot(mat: *mut matrix_t, read_cells: bool) -> Option<MatSnapshot> {
    if mat.is_null() {
        return None;
    }
    unsafe {
        let width = (*mat).width;
        let height = (*mat).height;
        let mut rows_null = Vec::new();
        let mut cells = Vec::new();
        if !(*mat).matrix.is_null() && height > 0 {
            for i in 0..height as isize {
                let row = *(*mat).matrix.offset(i);
                rows_null.push(row.is_null());
                let mut r = Vec::new();
                if read_cells && !row.is_null() && width > 0 {
                    for j in 0..width as isize {
                        r.push(*row.offset(j));
                    }
                }
                cells.push(r);
            }
        }
        Some(MatSnapshot {
            width,
            height,
            rows_null,
            cells,
        })
    }
}

/// Copies a C string out and frees the original with libc `free`, matching how
/// the C `driver` disposes of `matrix_to_string`'s result.
pub unsafe fn take_cstring(p: *mut c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    unsafe {
        let mut out = Vec::new();
        let mut q = p;
        while *q != 0 {
            out.push(*q as u8);
            q = q.add(1);
        }
        free(p as *mut u8);
        Some(out)
    }
}

/// Builds a matrix in `imp` by filling `allocate_matrix` cells directly.
pub unsafe fn make_matrix(imp: &Impl, width: c_int, height: c_int, vals: &[c_int]) -> *mut matrix_t {
    unsafe {
        let m = (imp.allocate_matrix)(width, height);
        assert!(!m.is_null(), "{}: allocate_matrix returned NULL", imp.name);
        assert_eq!(vals.len(), (width as usize) * (height as usize));
        for i in 0..height as isize {
            let row = *(*m).matrix.offset(i);
            for j in 0..width as isize {
                *row.offset(j) = vals[(i as usize) * (width as usize) + j as usize];
            }
        }
        m
    }
}
