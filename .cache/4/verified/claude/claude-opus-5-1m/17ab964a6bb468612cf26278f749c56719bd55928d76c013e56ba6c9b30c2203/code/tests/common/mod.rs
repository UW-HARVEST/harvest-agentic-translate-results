//! Shared differential-testing harness.
//!
//! Both the C shared library (`c_src/build/libdriver.so`) and the Rust shared
//! library (`target/{debug,release}/libdriver.so`) are loaded with `libloading`
//! and driven exclusively through their exported C symbols, exactly the way an
//! external consumer would.  No Rust function of the crate under test is ever
//! called directly, so the `#[no_mangle] extern "C"` wrappers are part of what
//! is being verified.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CStr, CString, OsStr};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits used by the harness itself (the test binary links the very same
// libc as both shared libraries, so `free()` here releases `malloc()`ations
// made inside either library).
// ---------------------------------------------------------------------------
unsafe extern "C" {
    pub fn free(ptr: *mut c_void);
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Mirror of `matrix_t` from `c_src/include/matrix.h`.
#[repr(C)]
#[derive(Debug)]
pub struct MatrixT {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

pub const EINVAL: c_int = 22;
pub const ENOENT: c_int = 2;
pub const EACCES: c_int = 13;
pub const EFAULT: c_int = 14;
pub const EISDIR: c_int = 21;
pub const ENOSPC: c_int = 28;
pub const ENAMETOOLONG: c_int = 36;
pub const EXIT_SUCCESS: c_int = 0;
pub const EXIT_FAILURE: c_int = 1;

type FnAllocateMatrix = unsafe extern "C" fn(c_int, c_int) -> *mut MatrixT;
type FnFreeMatrix = unsafe extern "C" fn(*mut MatrixT);
type FnInitFromString = unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut MatrixT;
type FnMultiply = unsafe extern "C" fn(*mut MatrixT, *mut MatrixT) -> *mut MatrixT;
type FnToString = unsafe extern "C" fn(*mut MatrixT) -> *mut c_char;
type FnWriteToFile = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type FnDriver =
    unsafe extern "C" fn(c_int, c_int, *const c_char, c_int, c_int, *const c_char) -> c_int;

/// The seven exported symbols of one implementation.
pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    pub allocate_matrix: FnAllocateMatrix,
    pub free_matrix: FnFreeMatrix,
    pub initialize_matrix_from_string: FnInitFromString,
    pub multiply_matrices: FnMultiply,
    pub matrix_to_string: FnToString,
    pub write_to_file: FnWriteToFile,
    pub driver: FnDriver,
}

unsafe impl Sync for Api {}
unsafe impl Send for Api {}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let debug = manifest_dir().join("target/debug/libdriver.so");
    if debug.exists() {
        return debug;
    }
    manifest_dir().join("target/release/libdriver.so")
}

/// Guards against testing a **stale** shared library: `cargo test` does not
/// rebuild the `cdylib` artifact, so a `cargo build` must have happened after the
/// last source change.  (Silently testing an outdated `.so` invalidates every
/// result, so this is a hard error.)
fn assert_not_stale(name: &str, path: &Path) {
    let lib_mtime = match fs::metadata(path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    // Only that library's *sources* are considered.  (`Cargo.toml` is left out on
    // purpose: a change that affects one profile does not make the other
    // profile's artifact stale, and cargo would not rebuild it.)
    let mut sources: Vec<PathBuf> = Vec::new();
    let src_dir = if name == "C" {
        manifest_dir().join("c_src")
    } else {
        manifest_dir().join("src")
    };
    let mut stack = vec![src_dir];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    if p.file_name().is_some_and(|n| n == "build") {
                        continue; // generated CMake output
                    }
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs" || x == "c" || x == "h") {
                    sources.push(p);
                }
            }
        }
    }
    for src in sources {
        if let Ok(t) = fs::metadata(&src).and_then(|m| m.modified()) {
            if newest.as_ref().map(|(n, _)| t > *n).unwrap_or(true) {
                newest = Some((t, src));
            }
        }
    }
    if let Some((t, src)) = newest {
        assert!(
            t <= lib_mtime,
            "{name} shared library {} is OLDER than {} — rebuild it first \
             (`cargo build` for the Rust .so, `cmake --build c_src/build` for the C .so)",
            path.display(),
            src.display()
        );
    }
}

fn load(name: &'static str, path: PathBuf) -> Api {
    assert!(
        path.exists(),
        "{name} shared library not found at {}.\n\
         Build the C library with:\n\
           cd c_src && mkdir -p build && cd build && \
           cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         Build the Rust library with: cargo build",
        path.display()
    );
    assert_not_stale(name, &path);
    // Leaked on purpose: the symbols are used for the whole lifetime of the
    // test process.
    let lib: &'static libloading::Library =
        Box::leak(Box::new(unsafe { libloading::Library::new(&path) }.unwrap_or_else(|e| {
            panic!("failed to dlopen {}: {e}", path.display())
        })));
    macro_rules! sym {
        ($t:ty, $n:literal) => {
            *unsafe { lib.get::<$t>($n) }
                .unwrap_or_else(|e| panic!("{} is missing symbol {:?}: {e}", name, $n))
        };
    }
    Api {
        name,
        path,
        allocate_matrix: sym!(FnAllocateMatrix, b"allocate_matrix\0"),
        free_matrix: sym!(FnFreeMatrix, b"free_matrix\0"),
        initialize_matrix_from_string: sym!(FnInitFromString, b"initialize_matrix_from_string\0"),
        multiply_matrices: sym!(FnMultiply, b"multiply_matrices\0"),
        matrix_to_string: sym!(FnToString, b"matrix_to_string\0"),
        write_to_file: sym!(FnWriteToFile, b"write_to_file\0"),
        driver: sym!(FnDriver, b"driver\0"),
    }
}

static C_API: OnceLock<Api> = OnceLock::new();
static RUST_API: OnceLock<Api> = OnceLock::new();

pub fn c_api() -> &'static Api {
    C_API.get_or_init(|| load("C", c_so_path()))
}

pub fn rust_api() -> &'static Api {
    RUST_API.get_or_init(|| load("Rust", rust_so_path()))
}

/// `(C, Rust)` — always call the C implementation first so that any global
/// state (errno, `stdio`) evolves identically for both.
pub fn both() -> (&'static Api, &'static Api) {
    (c_api(), rust_api())
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep failures reproducible.
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
    /// Uniform in `[lo, hi]` (inclusive).
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    pub fn i32_in(&mut self, lo: i32, hi: i32) -> i32 {
        self.range(lo as i64, hi as i64) as i32
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next_u64() % items.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// Snapshots of library results, for byte-exact comparison.
// ---------------------------------------------------------------------------

/// Observable state of a `matrix_t*` returned by one of the libraries.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MatSnap {
    pub is_null: bool,
    pub width: c_int,
    pub height: c_int,
    /// Row-major cell values (only read when `width > 0 && height > 0`).
    pub cells: Vec<c_int>,
    /// `true` when every row pointer was non-NULL.
    pub rows_non_null: bool,
}

impl MatSnap {
    pub fn null() -> Self {
        MatSnap {
            is_null: true,
            width: 0,
            height: 0,
            cells: Vec::new(),
            rows_non_null: false,
        }
    }
}

/// Reads the observable state of a matrix. `read_cells` must be `false` for
/// matrices whose cells were never written (freshly `allocate_matrix`d), since
/// their contents are indeterminate in the C original as well.
pub unsafe fn snap_matrix(p: *mut MatrixT, read_cells: bool) -> MatSnap {
    if p.is_null() {
        return MatSnap::null();
    }
    let m = unsafe { &*p };
    let mut cells = Vec::new();
    let mut rows_non_null = true;
    if m.height > 0 {
        for i in 0..m.height {
            let row = unsafe { *m.matrix.offset(i as isize) };
            if row.is_null() {
                rows_non_null = false;
                continue;
            }
            if read_cells {
                for j in 0..m.width {
                    cells.push(unsafe { *row.offset(j as isize) });
                }
            }
        }
    }
    MatSnap {
        is_null: false,
        width: m.width,
        height: m.height,
        cells,
        rows_non_null,
    }
}

/// Copies out a `char*` returned by one of the libraries and `free()`s it.
pub unsafe fn take_c_string(p: *mut c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(p) }.to_bytes().to_vec();
    unsafe { free(p as *mut c_void) };
    Some(bytes)
}

/// A NUL-terminated buffer usable as `const char*`; also supports "no string
/// at all" (NULL pointer).
pub struct CBuf(Option<CString>);

impl CBuf {
    pub fn new(bytes: impl AsRef<[u8]>) -> Self {
        CBuf(Some(CString::new(bytes.as_ref()).expect("input contains NUL")))
    }
    pub fn null() -> Self {
        CBuf(None)
    }
    pub fn as_ptr(&self) -> *const c_char {
        match &self.0 {
            Some(s) => s.as_ptr(),
            None => ptr::null(),
        }
    }
}

// ---------------------------------------------------------------------------
// stderr capture (fd 2 is process global ⇒ serialised by a mutex).
// ---------------------------------------------------------------------------
static STDERR_LOCK: Mutex<()> = Mutex::new(());

/// Runs `f` with the process' `stderr` redirected into a temporary file and
/// returns `(result, captured bytes)`.
pub fn capture_stderr<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let _guard = STDERR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = unique_path("stderr", ".txt");
    let file = fs::File::create(&path).expect("create stderr capture file");
    unsafe { fflush(ptr::null_mut()) };
    let saved = unsafe { dup(2) };
    assert!(saved >= 0, "dup(2) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 2) } >= 0, "dup2 failed");
    let result = f();
    unsafe { fflush(ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 2) } >= 0, "restore dup2 failed");
    unsafe { close(saved) };
    drop(file);
    let bytes = fs::read(&path).unwrap_or_default();
    let _ = fs::remove_file(&path);
    (result, bytes)
}

// ---------------------------------------------------------------------------
// Temporary files/directories (no external crates).
// ---------------------------------------------------------------------------
static COUNTER: AtomicU64 = AtomicU64::new(0);
static TMP_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Per-process scratch directory.
///
/// The name mixes the PID *and* a high-resolution timestamp on purpose: PIDs
/// are recycled, and a leftover directory from an earlier run would otherwise
/// be inherited together with its contents (e.g. the `matrix.txt` **directory**
/// that the E30 test plants in its working directory), which makes unrelated
/// tests fail spuriously.
fn harness_tmp_root() -> &'static Path {
    TMP_ROOT.get_or_init(|| {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "driver-difftest-{}-{stamp:x}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create temp root");
        root
    })
}

pub fn unique_path(prefix: &str, suffix: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    harness_tmp_root().join(format!("{prefix}-{n}{suffix}"))
}

/// Creates a brand-new, guaranteed-empty temporary directory.
pub fn unique_dir(prefix: &str) -> PathBuf {
    for _ in 0..1000 {
        let p = unique_path(prefix, "");
        match fs::create_dir(&p) {
            Ok(()) => return p,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => panic!("create temp dir {}: {e}", p.display()),
        }
    }
    panic!("could not create a fresh temporary directory");
}

pub fn path_cbuf(p: &Path) -> CBuf {
    CBuf::new(p.as_os_str().as_bytes())
}

pub fn os_str(bytes: &[u8]) -> &OsStr {
    OsStr::from_bytes(bytes)
}

// ---------------------------------------------------------------------------
// Working-directory control (needed by `driver`, which hardcodes
// `OUT_FILE "matrix.txt"` relative to the CWD).
// ---------------------------------------------------------------------------
static CWD_LOCK: Mutex<()> = Mutex::new(());

pub fn cwd_lock() -> MutexGuard<'static, ()> {
    CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Serialises on the CWD, chdirs into a fresh temp directory, runs `f`, then
/// restores the previous working directory.
pub fn in_temp_cwd<R>(f: impl FnOnce(&Path) -> R) -> R {
    let _guard = cwd_lock();
    let prev = std::env::current_dir().expect("getcwd");
    let dir = unique_dir("cwd");
    std::env::set_current_dir(&dir).expect("chdir");
    let out = f(&dir);
    std::env::set_current_dir(&prev).expect("chdir back");
    out
}

// ---------------------------------------------------------------------------
// Differential helpers.
// ---------------------------------------------------------------------------

/// Byte-exact comparison with a readable failure message.
pub fn assert_bytes_eq(c_bytes: &[u8], rust_bytes: &[u8], what: &str) {
    if c_bytes != rust_bytes {
        panic!(
            "{what}\n  C   ({} bytes): {:?}\n  Rust({} bytes): {:?}",
            c_bytes.len(),
            String::from_utf8_lossy(c_bytes),
            rust_bytes.len(),
            String::from_utf8_lossy(rust_bytes)
        );
    }
}

/// Byte-exact comparison of two optional strings (NULL vs non-NULL included).
pub fn assert_opt_bytes_eq(c_bytes: &Option<Vec<u8>>, rust_bytes: &Option<Vec<u8>>, what: &str) {
    match (c_bytes, rust_bytes) {
        (None, None) => {}
        (Some(a), Some(b)) => assert_bytes_eq(a, b, what),
        (a, b) => panic!(
            "{what}: NULL-ness mismatch: C = {:?}, Rust = {:?}",
            a.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
            b.as_ref().map(|v| String::from_utf8_lossy(v).into_owned())
        ),
    }
}

/// Builds the "W H\n"-style row/column text for a matrix of values.
pub fn matrix_text(values: &[Vec<i32>]) -> String {
    let mut s = String::new();
    for row in values {
        let cols: Vec<String> = row.iter().map(|v| v.to_string()).collect();
        s.push_str(&cols.join(" "));
        s.push('\n');
    }
    s
}

/// Reference rendering of `matrix_to_string` (only used for sanity checks; the
/// authoritative comparison is always C vs Rust).
pub fn expected_to_string(values: &[Vec<i32>]) -> String {
    matrix_text(values)
}

pub fn random_values(rng: &mut Rng, width: i32, height: i32, lo: i32, hi: i32) -> Vec<Vec<i32>> {
    (0..height.max(0))
        .map(|_| (0..width.max(0)).map(|_| rng.i32_in(lo, hi)).collect())
        .collect()
}

/// Loads a matrix through `initialize_matrix_from_string` on both libraries and
/// asserts the observable results are identical; returns the two pointers (the
/// caller owns them and must `free_matrix` them with the matching library).
pub unsafe fn init_both(
    text: &CBuf,
    width: c_int,
    height: c_int,
    context: &str,
) -> (*mut MatrixT, *mut MatrixT) {
    let (c, r) = both();
    let ((cp, c_err), (rp, r_err)) = (
        capture_stderr(|| unsafe { (c.initialize_matrix_from_string)(text.as_ptr(), width, height) }),
        capture_stderr(|| unsafe { (r.initialize_matrix_from_string)(text.as_ptr(), width, height) }),
    );
    let cs = unsafe { snap_matrix(cp, true) };
    let rs = unsafe { snap_matrix(rp, true) };
    assert_eq!(cs, rs, "initialize_matrix_from_string mismatch [{context}]");
    assert_bytes_eq(
        &c_err,
        &r_err,
        &format!("initialize_matrix_from_string stderr mismatch [{context}]"),
    );
    (cp, rp)
}
