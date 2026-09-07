//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported C symbols — the Rust crate is never
//! linked directly, so the `#[no_mangle] extern "C"` wrappers are part of what
//! is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits the harness itself needs
// ---------------------------------------------------------------------------

extern "C" {
    fn free(ptr: *mut c_void);
    fn malloc(size: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// `free()` from the process-wide libc — the same allocator both `.so`s use.
pub unsafe fn libc_free(p: *mut c_void) {
    free(p)
}

pub unsafe fn libc_malloc(n: usize) -> *mut c_void {
    malloc(n)
}

// ---------------------------------------------------------------------------
// The C `matrix_t`
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct { int** matrix; int width; int height; } matrix_t;
/// ```
#[repr(C)]
pub struct MatrixT {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

// ---------------------------------------------------------------------------
// Loaded API surface
// ---------------------------------------------------------------------------

pub type FnAllocateMatrix = unsafe extern "C" fn(c_int, c_int) -> *mut MatrixT;
pub type FnFreeMatrix = unsafe extern "C" fn(*mut MatrixT);
pub type FnInitFromString = unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut MatrixT;
pub type FnMultiply = unsafe extern "C" fn(*mut MatrixT, *mut MatrixT) -> *mut MatrixT;
pub type FnToString = unsafe extern "C" fn(*mut MatrixT) -> *mut c_char;
pub type FnWriteToFile = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
pub type FnDriver =
    unsafe extern "C" fn(c_int, c_int, *const c_char, c_int, c_int, *const c_char) -> c_int;

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

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFFTEST_C_SO") {
        return PathBuf::from(p);
    }
    workspace_root().join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFFTEST_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let release = base.join("release/libdriver.so");
    if release.exists() {
        return release;
    }
    base.join("debug/libdriver.so")
}

/// `cargo test` does not necessarily rebuild the `cdylib`, so a stale
/// `libdriver.so` would silently be tested instead of the current sources.
/// Refuse to run in that case.
fn assert_fresh(path: &Path, sources: &Path) {
    let Ok(so_meta) = std::fs::metadata(path) else {
        return;
    };
    let Ok(so_time) = so_meta.modified() else {
        return;
    };
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let Ok(rd) = std::fs::read_dir(sources) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("rs")
            && p.extension().and_then(|s| s.to_str()) != Some("c")
            && p.extension().and_then(|s| s.to_str()) != Some("h")
        {
            continue;
        }
        if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
            if newest.as_ref().is_none_or(|(n, _)| t > *n) {
                newest = Some((t, p));
            }
        }
    }
    if let Some((t, p)) = newest {
        assert!(
            t <= so_time,
            "{} is STALE: {} is newer. Rebuild it \
             (cargo build --release  /  cmake --build c_src/build) before running the tests.",
            path.display(),
            p.display()
        );
    }
}

unsafe fn load(name: &'static str, path: PathBuf) -> Api {
    assert!(
        path.exists(),
        "shared object {} not found — build it first \
         (cd c_src/build && cmake --build .  /  cd translation && cargo build --release)",
        path.display()
    );
    let lib = Library::new(&path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
    // Leaked on purpose: the resolved function pointers must stay valid for the
    // whole test-binary lifetime.
    let lib: &'static Library = Box::leak(Box::new(lib));

    macro_rules! sym {
        ($t:ty, $n:literal) => {{
            let s: Symbol<$t> = lib
                .get(concat!($n, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("{} missing symbol {}: {e}", name, $n));
            *s
        }};
    }

    Api {
        name,
        path,
        allocate_matrix: sym!(FnAllocateMatrix, "allocate_matrix"),
        free_matrix: sym!(FnFreeMatrix, "free_matrix"),
        initialize_matrix_from_string: sym!(FnInitFromString, "initialize_matrix_from_string"),
        multiply_matrices: sym!(FnMultiply, "multiply_matrices"),
        matrix_to_string: sym!(FnToString, "matrix_to_string"),
        write_to_file: sym!(FnWriteToFile, "write_to_file"),
        driver: sym!(FnDriver, "driver"),
    }
}

static C_API: OnceLock<Api> = OnceLock::new();
static RUST_API: OnceLock<Api> = OnceLock::new();

pub fn c_api() -> &'static Api {
    C_API.get_or_init(|| unsafe {
        let p = c_so_path();
        assert_fresh(&p, &workspace_root().join("c_src/src"));
        assert_fresh(&p, &workspace_root().join("c_src/include"));
        load("C", p)
    })
}

pub fn rust_api() -> &'static Api {
    RUST_API.get_or_init(|| unsafe {
        let p = rust_so_path();
        assert_fresh(&p, &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
        load("Rust", p)
    })
}

/// `(C, Rust)` — always call the C one first so any allocator-state ordering
/// matches the C-only baseline.
pub fn both() -> (&'static Api, &'static Api) {
    (c_api(), rust_api())
}

// ---------------------------------------------------------------------------
// Global serialisation
//
// `stderr` redirection (fd 2), the process CWD and the shared heap are all
// process-global, so every test takes this lock.
// ---------------------------------------------------------------------------

static GLOBAL: Mutex<()> = Mutex::new(());

pub fn global_lock() -> MutexGuard<'static, ()> {
    limit_address_space();
    match GLOBAL.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// Address-space cap
//
// Several `ERRORS.md` rows are reached by handing `allocate_matrix` a negative
// or enormous dimension so that `malloc` fails. Without a cap, Linux
// overcommit lets some of those requests *succeed* and the process is then
// OOM-killed — which tells us nothing about C-vs-Rust parity. Capping
// RLIMIT_AS makes those `malloc`s fail deterministically and identically for
// both libraries (they share one address space), while still leaving far more
// headroom than any valid-path test needs.
// ---------------------------------------------------------------------------

#[repr(C)]
struct RLimit {
    rlim_cur: u64,
    rlim_max: u64,
}

const RLIMIT_AS: c_int = 9; // Linux
const AS_CAP: u64 = 3 << 30; // 3 GiB

extern "C" {
    fn getrlimit(resource: c_int, rlim: *mut RLimit) -> c_int;
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
}

static LIMITED: OnceLock<()> = OnceLock::new();

pub fn limit_address_space() {
    LIMITED.get_or_init(|| unsafe {
        let mut cur = RLimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if getrlimit(RLIMIT_AS, &mut cur) != 0 {
            return;
        }
        let cap = if cur.rlim_max == u64::MAX {
            AS_CAP
        } else {
            cur.rlim_max.min(AS_CAP)
        };
        let want = RLimit {
            rlim_cur: cap,
            rlim_max: cur.rlim_max,
        };
        let _ = setrlimit(RLIMIT_AS, &want);
    });
}

// ---------------------------------------------------------------------------
// Scratch directory (inside the crate's target/ so it is always writable)
// ---------------------------------------------------------------------------

pub fn scratch_dir(tag: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("difftest")
        .join(tag);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("create scratch dir");
    d
}

// ---------------------------------------------------------------------------
// stderr capture
// ---------------------------------------------------------------------------

/// Run `f` with fd 2 redirected to a temporary file; return `f`'s value and the
/// bytes it wrote to stderr.
pub fn capture_stderr<R>(tag: &str, f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("difftest-stderr");
    std::fs::create_dir_all(&dir).expect("create stderr capture dir");
    let path = dir.join(format!("{tag}.txt"));

    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(2);
        assert!(saved >= 0, "dup(2) failed");

        let file = std::fs::File::create(&path).expect("create stderr capture file");
        assert!(dup2(file.as_raw_fd(), 2) >= 0, "dup2 failed");
        drop(file);

        let r = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 2) >= 0, "restore dup2 failed");
        close(saved);

        let bytes = std::fs::read(&path).unwrap_or_default();
        (r, bytes)
    }
}

// ---------------------------------------------------------------------------
// Snapshots
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Snap {
    pub width: c_int,
    pub height: c_int,
    /// Row-major cells, only captured when the dimensions are sane and small.
    pub cells: Option<Vec<c_int>>,
}

/// Read the observable state of a `matrix_t*` without assuming it is readable
/// beyond what its own `width`/`height` claim.
pub unsafe fn snapshot(mat: *const MatrixT) -> Option<Snap> {
    if mat.is_null() {
        return None;
    }
    let width = (*mat).width;
    let height = (*mat).height;
    let cells = if width >= 0 && height >= 0 && (width as i64) * (height as i64) <= (1 << 22) {
        let mut v = Vec::with_capacity((width as usize) * (height as usize));
        for i in 0..height {
            let row = *(*mat).matrix.offset(i as isize);
            for j in 0..width {
                v.push(*row.offset(j as isize));
            }
        }
        Some(v)
    } else {
        None
    };
    Some(Snap {
        width,
        height,
        cells,
    })
}

/// Take ownership of a `char*` the library returned, copy the bytes, `free` it.
pub unsafe fn take_cstring(p: *mut c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    let n = strlen(p);
    let mut v = Vec::with_capacity(n);
    v.extend_from_slice(std::slice::from_raw_parts(p as *const u8, n));
    free(p as *mut c_void);
    Some(v)
}

pub fn cs(s: &str) -> CString {
    CString::new(s).expect("no interior NUL")
}

pub fn cs_bytes(b: &[u8]) -> CString {
    CString::new(b).expect("no interior NUL")
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }
    /// Inclusive range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    pub fn range_usize(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + self.below((hi - lo + 1) as u64) as usize
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}

/// The fixed seed used by every property-style row, so failures reproduce.
pub const SEED: u64 = 0x2545F4914F6CDD1D;

// ---------------------------------------------------------------------------
// Matrix-string builders
// ---------------------------------------------------------------------------

/// Render `height` rows of `width` values, single-space separated, no trailing
/// newline (the "exact fit" shape).
pub fn render(cells: &[i32], width: usize, height: usize) -> String {
    let mut s = String::new();
    for i in 0..height {
        for j in 0..width {
            if j > 0 {
                s.push(' ');
            }
            s.push_str(&cells[i * width + j].to_string());
        }
        if i + 1 != height {
            s.push('\n');
        }
    }
    s
}

pub fn random_cells(rng: &mut Rng, n: usize, lo: i32, hi: i32) -> Vec<i32> {
    (0..n).map(|_| rng.range_i32(lo, hi)).collect()
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

#[track_caller]
pub fn assert_eq_bytes(ctx: &str, c: &Option<Vec<u8>>, r: &Option<Vec<u8>>) {
    match (c, r) {
        (None, None) => {}
        (Some(a), Some(b)) => assert!(
            a == b,
            "{ctx}: string mismatch\n  C   : {}\n  Rust: {}",
            show(a),
            show(b)
        ),
        _ => panic!(
            "{ctx}: NULL-ness mismatch: C={:?} Rust={:?}",
            c.as_ref().map(|v| show(v)),
            r.as_ref().map(|v| show(v))
        ),
    }
}

#[track_caller]
pub fn assert_eq_snap(ctx: &str, c: &Option<Snap>, r: &Option<Snap>) {
    assert!(
        c == r,
        "{ctx}: matrix mismatch\n  C   : {c:?}\n  Rust: {r:?}"
    );
}

#[track_caller]
pub fn assert_eq_stderr(ctx: &str, c: &[u8], r: &[u8]) {
    assert!(
        c == r,
        "{ctx}: stderr mismatch\n  C   : {}\n  Rust: {}",
        show(c),
        show(r)
    );
}

// ---------------------------------------------------------------------------
// Convenience: build a matrix with a library, snapshot it, free it
// ---------------------------------------------------------------------------

/// `initialize_matrix_from_string` + snapshot + `matrix_to_string` + free, all
/// with the same library. Returns `(snapshot, rendered string, stderr)`.
pub fn init_and_render(
    api: &Api,
    tag: &str,
    input: &CStr,
    width: c_int,
    height: c_int,
) -> (Option<Snap>, Option<Vec<u8>>, Vec<u8>) {
    let ((snap, s), err) = capture_stderr(tag, || unsafe {
        let m = (api.initialize_matrix_from_string)(input.as_ptr(), width, height);
        let snap = snapshot(m);
        let s = take_cstring((api.matrix_to_string)(m));
        (api.free_matrix)(m);
        (snap, s)
    });
    (snap, s, err)
}
