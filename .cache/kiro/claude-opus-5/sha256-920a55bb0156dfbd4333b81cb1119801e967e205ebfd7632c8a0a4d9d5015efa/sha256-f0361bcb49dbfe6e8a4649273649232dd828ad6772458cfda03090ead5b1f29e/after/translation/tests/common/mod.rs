//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls only their exported
//! C symbols — the Rust implementation is never called directly, so the
//! `#[no_mangle]` wrappers and the `#[repr(C)]` struct layout are under test too.

#![allow(dead_code)]

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits the harness itself needs
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn free(p: *mut c_void);
    fn fflush(f: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn strlen(s: *const c_char) -> usize;
}

/// libc `free` — both `.so`s allocate with libc `malloc`, so the caller frees
/// their returned pointers this way (this is itself part of the contract).
pub unsafe fn libc_free(p: *mut c_void) {
    unsafe { free(p) }
}

// ---------------------------------------------------------------------------
// The C struct under test
// ---------------------------------------------------------------------------

/// `typedef struct { int** matrix; int width; int height; } matrix_t;`
#[repr(C)]
pub struct MatrixT {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

/// Owned snapshot of a `matrix_t` read back across the FFI boundary.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MatrixSnapshot {
    pub width: c_int,
    pub height: c_int,
    /// `None` when `mat->matrix` is NULL, otherwise one entry per row; a row is
    /// `None` when that row pointer is NULL.
    pub rows: Option<Vec<Option<Vec<c_int>>>>,
}

/// Read a `matrix_t*` back into an owned snapshot.
///
/// `read_values` is false for `allocate_matrix` results, whose element bytes are
/// uninitialised `malloc` memory and therefore legitimately differ.
pub unsafe fn snapshot(mat: *const MatrixT, read_values: bool) -> Option<MatrixSnapshot> {
    if mat.is_null() {
        return None;
    }
    unsafe {
        let width = (*mat).width;
        let height = (*mat).height;
        let rows = if (*mat).matrix.is_null() {
            None
        } else {
            let mut v = Vec::new();
            let n = if height > 0 { height } else { 0 };
            for i in 0..n {
                let row = *(*mat).matrix.offset(i as isize);
                if row.is_null() {
                    v.push(None);
                } else if read_values && width > 0 {
                    let mut r = Vec::with_capacity(width as usize);
                    for j in 0..width {
                        r.push(*row.offset(j as isize));
                    }
                    v.push(Some(r));
                } else {
                    v.push(Some(Vec::new()));
                }
            }
            Some(v)
        };
        Some(MatrixSnapshot { width, height, rows })
    }
}

/// Probe that every row slot of an `allocate_matrix` result is a usable
/// allocation of `width` ints, without comparing its uninitialised contents.
pub unsafe fn probe_writable(mat: *const MatrixT) -> Vec<bool> {
    let mut out = Vec::new();
    if mat.is_null() {
        return out;
    }
    unsafe {
        if (*mat).matrix.is_null() {
            return out;
        }
        let h = (*mat).height;
        let w = (*mat).width;
        for i in 0..h.max(0) {
            let row = *(*mat).matrix.offset(i as isize);
            if row.is_null() {
                out.push(false);
                continue;
            }
            for j in 0..w.max(0) {
                *row.offset(j as isize) = (i * 31 + j) as c_int;
            }
            let mut ok = true;
            for j in 0..w.max(0) {
                if *row.offset(j as isize) != (i * 31 + j) as c_int {
                    ok = false;
                }
            }
            out.push(ok);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

type FnAllocate = unsafe extern "C" fn(c_int, c_int) -> *mut MatrixT;
type FnFree = unsafe extern "C" fn(*mut MatrixT);
type FnInit = unsafe extern "C" fn(*const c_char, c_int, c_int) -> *mut MatrixT;
type FnMultiply = unsafe extern "C" fn(*mut MatrixT, *mut MatrixT) -> *mut MatrixT;
type FnToString = unsafe extern "C" fn(*mut MatrixT) -> *mut c_char;
type FnWrite = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type FnDriver =
    unsafe extern "C" fn(c_int, c_int, *const c_char, c_int, c_int, *const c_char) -> c_int;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    lib: Library,
}

impl Impl {
    fn open(name: &'static str, path: &PathBuf) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", name, path.display()));
        Impl { name, lib }
    }

    fn sym<T>(&self, name: &[u8]) -> Symbol<'_, T> {
        unsafe { self.lib.get::<T>(name) }.unwrap_or_else(|e| {
            panic!(
                "{} does not export `{}`: {e}",
                self.name,
                String::from_utf8_lossy(&name[..name.len() - 1])
            )
        })
    }

    pub unsafe fn allocate_matrix(&self, w: c_int, h: c_int) -> *mut MatrixT {
        let f: Symbol<FnAllocate> = self.sym(b"allocate_matrix\0");
        unsafe { f(w, h) }
    }

    pub unsafe fn free_matrix(&self, m: *mut MatrixT) {
        let f: Symbol<FnFree> = self.sym(b"free_matrix\0");
        unsafe { f(m) }
    }

    pub unsafe fn initialize_matrix_from_string(
        &self,
        s: *const c_char,
        w: c_int,
        h: c_int,
    ) -> *mut MatrixT {
        let f: Symbol<FnInit> = self.sym(b"initialize_matrix_from_string\0");
        unsafe { f(s, w, h) }
    }

    pub unsafe fn multiply_matrices(&self, a: *mut MatrixT, b: *mut MatrixT) -> *mut MatrixT {
        let f: Symbol<FnMultiply> = self.sym(b"multiply_matrices\0");
        unsafe { f(a, b) }
    }

    pub unsafe fn matrix_to_string(&self, m: *mut MatrixT) -> *mut c_char {
        let f: Symbol<FnToString> = self.sym(b"matrix_to_string\0");
        unsafe { f(m) }
    }

    pub unsafe fn write_to_file(&self, name: *const c_char, content: *const c_char) -> c_int {
        let f: Symbol<FnWrite> = self.sym(b"write_to_file\0");
        unsafe { f(name, content) }
    }

    pub unsafe fn driver(
        &self,
        wa: c_int,
        ha: c_int,
        ma: *const c_char,
        wb: c_int,
        hb: c_int,
        mb: *const c_char,
    ) -> c_int {
        let f: Symbol<FnDriver> = self.sym(b"driver\0");
        unsafe { f(wa, ha, ma, wb, hb, mb) }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
static GUARD: Mutex<()> = Mutex::new(());

/// Serialises tests: `stderr` capture (fd 2) and `chdir` are process-global.
pub fn lock() -> MutexGuard<'static, ()> {
    GUARD.lock().unwrap_or_else(|e| e.into_inner())
}

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.push("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not built: {} — run the cmake build first",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    // `cargo test` does not build a `cdylib` artifact, so the `.so` must be
    // produced by an explicit `cargo build` (see `run_tests.sh`). Honour an
    // override first, then look next to the test binary, then in target/*.
    if let Some(p) = std::env::var_os("RUST_DRIVER_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_DRIVER_SO does not exist: {}", p.display());
        return p;
    }
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    for _ in 0..3 {
        let cand = dir.join("libdriver.so");
        if cand.exists() {
            return cand;
        }
        if !dir.pop() {
            break;
        }
    }
    for profile in ["debug", "release"] {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("target");
        p.push(profile);
        p.push("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "could not locate the Rust libdriver.so (searched near {} and target/{{debug,release}}) \
         — run `cargo build` / `cargo build --release` first, or set RUST_DRIVER_SO",
        exe.display()
    );
}

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::open("C libdriver.so", &c_so_path()),
        rs: Impl::open("Rust libdriver.so", &rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// stderr capture
// ---------------------------------------------------------------------------

/// Run `f` with fd 2 redirected to a temp file; return `(result, stderr_bytes)`.
pub fn capture_stderr<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let path = unique_tmp_path("stderr");
    let file = std::fs::File::create(&path).expect("create stderr capture file");
    let saved = unsafe { dup(2) };
    assert!(saved >= 0, "dup(2) failed");
    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(file.as_raw_fd(), 2) >= 0, "dup2 failed");
    }
    let r = f();
    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 2);
        close(saved);
    }
    drop(file);
    let bytes = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    (r, bytes)
}

static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn unique_tmp_path(tag: &str) -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let mut p = std::env::temp_dir();
    p.push(format!("cdiff-{}-{}-{}", std::process::id(), tag, n));
    p
}

// ---------------------------------------------------------------------------
// C string helpers
// ---------------------------------------------------------------------------

pub fn cs(s: &str) -> CString {
    CString::new(s).expect("no interior NUL")
}

/// Take ownership of a `char*` returned by either `.so`, freeing it with libc
/// `free` (which is what a C consumer does, and what the C `.so` requires).
pub unsafe fn take_c_string(p: *mut c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    unsafe {
        let len = strlen(p);
        let bytes = std::slice::from_raw_parts(p as *const u8, len).to_vec();
        libc_free(p as *mut c_void);
        Some(bytes)
    }
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

pub fn show_opt(b: &Option<Vec<u8>>) -> String {
    match b {
        None => "<NULL>".to_string(),
        Some(v) => format!("{:?}", show(v)),
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed, no external crates)
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    pub fn i32_bounded(&mut self, bound: i32) -> i32 {
        self.range(-(bound as i64), bound as i64) as i32
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

// ---------------------------------------------------------------------------
// High-level differential comparisons
// ---------------------------------------------------------------------------

/// `initialize_matrix_from_string` on both, compare struct contents + stderr.
pub fn diff_init(label: &str, input: &str, w: c_int, h: c_int) {
    let p = pair();
    let cstr = cs(input);
    let (c_snap, c_err) = capture_stderr(|| unsafe {
        let m = p.c.initialize_matrix_from_string(cstr.as_ptr(), w, h);
        let s = snapshot(m, true);
        p.c.free_matrix(m);
        s
    });
    let (r_snap, r_err) = capture_stderr(|| unsafe {
        let m = p.rs.initialize_matrix_from_string(cstr.as_ptr(), w, h);
        let s = snapshot(m, true);
        p.rs.free_matrix(m);
        s
    });
    assert_eq!(
        c_snap, r_snap,
        "[{label}] initialize_matrix_from_string({input:?}, {w}, {h}) diverged\n C: {c_snap:?}\nRS: {r_snap:?}"
    );
    assert_eq!(
        show(&c_err),
        show(&r_err),
        "[{label}] stderr diverged for initialize_matrix_from_string({input:?}, {w}, {h})"
    );
}

/// `initialize_matrix_from_string` × 2 → `multiply_matrices` → snapshot, on both.
pub fn diff_multiply(
    label: &str,
    a: &str,
    wa: c_int,
    ha: c_int,
    b: &str,
    wb: c_int,
    hb: c_int,
) {
    let p = pair();
    let (ca, cb) = (cs(a), cs(b));
    let run = |imp: &Impl| unsafe {
        let ma = imp.initialize_matrix_from_string(ca.as_ptr(), wa, ha);
        let mb = imp.initialize_matrix_from_string(cb.as_ptr(), wb, hb);
        let res = if ma.is_null() || mb.is_null() {
            std::ptr::null_mut()
        } else {
            imp.multiply_matrices(ma, mb)
        };
        let snap = snapshot(res, true);
        imp.free_matrix(res);
        imp.free_matrix(ma);
        imp.free_matrix(mb);
        snap
    };
    let (c_snap, c_err) = capture_stderr(|| run(&p.c));
    let (r_snap, r_err) = capture_stderr(|| run(&p.rs));
    assert_eq!(
        c_snap, r_snap,
        "[{label}] multiply diverged for A({wa}x{ha})={a:?} B({wb}x{hb})={b:?}\n C: {c_snap:?}\nRS: {r_snap:?}"
    );
    assert_eq!(show(&c_err), show(&r_err), "[{label}] stderr diverged (multiply)");
}

/// `initialize_matrix_from_string` → `matrix_to_string`, on both.
pub fn diff_to_string(label: &str, input: &str, w: c_int, h: c_int) {
    let p = pair();
    let cstr = cs(input);
    let run = |imp: &Impl| unsafe {
        let m = imp.initialize_matrix_from_string(cstr.as_ptr(), w, h);
        let s = imp.matrix_to_string(m);
        let out = take_c_string(s);
        imp.free_matrix(m);
        out
    };
    let (c_out, c_err) = capture_stderr(|| run(&p.c));
    let (r_out, r_err) = capture_stderr(|| run(&p.rs));
    assert_eq!(
        c_out,
        r_out,
        "[{label}] matrix_to_string diverged for ({w}x{h}) {input:?}\n C: {}\nRS: {}",
        show_opt(&c_out),
        show_opt(&r_out)
    );
    assert_eq!(show(&c_err), show(&r_err), "[{label}] stderr diverged (to_string)");
}

/// Full pipeline: init × 2 → multiply → matrix_to_string, on both.
pub fn diff_pipeline(
    label: &str,
    a: &str,
    wa: c_int,
    ha: c_int,
    b: &str,
    wb: c_int,
    hb: c_int,
) {
    let p = pair();
    let (ca, cb) = (cs(a), cs(b));
    let run = |imp: &Impl| unsafe {
        let ma = imp.initialize_matrix_from_string(ca.as_ptr(), wa, ha);
        let mb = imp.initialize_matrix_from_string(cb.as_ptr(), wb, hb);
        let res = if ma.is_null() || mb.is_null() {
            std::ptr::null_mut()
        } else {
            imp.multiply_matrices(ma, mb)
        };
        let s = imp.matrix_to_string(res);
        let out = take_c_string(s);
        imp.free_matrix(res);
        imp.free_matrix(ma);
        imp.free_matrix(mb);
        out
    };
    let (c_out, c_err) = capture_stderr(|| run(&p.c));
    let (r_out, r_err) = capture_stderr(|| run(&p.rs));
    assert_eq!(
        c_out,
        r_out,
        "[{label}] pipeline diverged A({wa}x{ha})={a:?} B({wb}x{hb})={b:?}\n C: {}\nRS: {}",
        show_opt(&c_out),
        show_opt(&r_out)
    );
    assert_eq!(show(&c_err), show(&r_err), "[{label}] stderr diverged (pipeline)");
}

/// `write_to_file` on both, into two distinct paths; compare rc, errno-style
/// return, stderr and the resulting file bytes.
pub fn diff_write(label: &str, filename_suffix: &str, content: &[u8]) {
    let p = pair();
    let c_path = unique_tmp_path(&format!("w-c-{filename_suffix}"));
    let r_path = unique_tmp_path(&format!("w-rs-{filename_suffix}"));
    let c_name = cs(c_path.to_str().unwrap());
    let r_name = cs(r_path.to_str().unwrap());
    let content_c = CString::new(content).expect("no interior NUL");

    let (c_rc, c_err) = capture_stderr(|| unsafe {
        p.c.write_to_file(c_name.as_ptr(), content_c.as_ptr())
    });
    let (r_rc, r_err) = capture_stderr(|| unsafe {
        p.rs.write_to_file(r_name.as_ptr(), content_c.as_ptr())
    });

    assert_eq!(c_rc, r_rc, "[{label}] write_to_file return code diverged");
    let c_bytes = std::fs::read(&c_path).unwrap_or_default();
    let r_bytes = std::fs::read(&r_path).unwrap_or_default();
    assert_eq!(
        c_bytes.len(),
        r_bytes.len(),
        "[{label}] written length diverged ({} vs {})",
        c_bytes.len(),
        r_bytes.len()
    );
    assert!(c_bytes == r_bytes, "[{label}] written bytes diverged");
    // stderr messages embed the filename, which differs by construction; only
    // compare them when both are empty (the success path).
    assert_eq!(
        c_err.is_empty(),
        r_err.is_empty(),
        "[{label}] stderr presence diverged: C={:?} RS={:?}",
        show(&c_err),
        show(&r_err)
    );
    if c_rc == 0 {
        assert!(c_err.is_empty() && r_err.is_empty(), "[{label}] unexpected stderr on success");
    }
    let _ = std::fs::remove_file(&c_path);
    let _ = std::fs::remove_file(&r_path);
}

/// Run `driver` on both in isolated temp cwds; compare rc + `matrix.txt` bytes.
pub fn diff_driver(
    label: &str,
    wa: c_int,
    ha: c_int,
    a: &str,
    wb: c_int,
    hb: c_int,
    b: &str,
) {
    let p = pair();
    let (ca, cb) = (cs(a), cs(b));
    let orig = std::env::current_dir().expect("cwd");

    let run = |imp: &Impl, tag: &str| {
        let dir = unique_tmp_path(&format!("drv-{tag}"));
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::env::set_current_dir(&dir).expect("chdir");
        let (rc, err) = capture_stderr(|| unsafe {
            imp.driver(wa, ha, ca.as_ptr(), wb, hb, cb.as_ptr())
        });
        let out = std::fs::read(dir.join("matrix.txt")).ok();
        std::env::set_current_dir(&orig).expect("restore cwd");
        let _ = std::fs::remove_dir_all(&dir);
        (rc, err, out)
    };

    let (c_rc, c_err, c_out) = run(&p.c, "c");
    let (r_rc, r_err, r_out) = run(&p.rs, "rs");

    assert_eq!(
        c_rc, r_rc,
        "[{label}] driver return code diverged (C={c_rc} RS={r_rc})"
    );
    assert_eq!(
        c_out,
        r_out,
        "[{label}] driver matrix.txt diverged\n C: {}\nRS: {}",
        show_opt(&c_out),
        show_opt(&r_out)
    );
    assert_eq!(show(&c_err), show(&r_err), "[{label}] driver stderr diverged");
}

/// The largest element magnitude that keeps `matrix_to_string` inside the C's
/// own `11*width`-per-row allocation.
///
/// Total needed is `Σ len(v) + h*w + 1`, total allocated is `11*h*w + h + 1`, so
/// the write stays in bounds as long as every element renders in ≤ 10
/// characters. Positive `int`s always do (`2147483647` is 10 chars); a negative
/// needs ≥ 9 digits of headroom for the sign, i.e. `v >= -999_999_999`.
/// Anything below that heap-overflows **in the C** (see the UB note in
/// `ERRORS.md`), so those cases are skipped rather than compared.
pub const MIN_STRING_SAFE: c_int = -999_999_999;

pub fn value_is_string_safe(v: c_int) -> bool {
    v >= MIN_STRING_SAFE
}

fn snapshot_is_string_safe(s: &Option<MatrixSnapshot>) -> bool {
    match s {
        None => true,
        Some(m) => match &m.rows {
            None => true,
            Some(rows) => rows.iter().all(|r| match r {
                None => true,
                Some(v) => v.iter().all(|&x| value_is_string_safe(x)),
            }),
        },
    }
}

/// Would `matrix_to_string` on `initialize_matrix_from_string(input, w, h)` stay
/// inside the C's buffer? Determined by asking the **C** library itself.
pub fn init_is_string_safe(input: &str, w: c_int, h: c_int) -> bool {
    let p = pair();
    let cstr = cs(input);
    let (snap, _) = capture_stderr(|| unsafe {
        let m = p.c.initialize_matrix_from_string(cstr.as_ptr(), w, h);
        let s = snapshot(m, true);
        p.c.free_matrix(m);
        s
    });
    snapshot_is_string_safe(&snap)
}

/// Same question for the product of two matrices, asked of the C library.
pub fn product_is_string_safe(
    a: &str,
    wa: c_int,
    ha: c_int,
    b: &str,
    wb: c_int,
    hb: c_int,
) -> bool {
    let p = pair();
    let (ca, cb) = (cs(a), cs(b));
    let (snap, _) = capture_stderr(|| unsafe {
        let ma = p.c.initialize_matrix_from_string(ca.as_ptr(), wa, ha);
        let mb = p.c.initialize_matrix_from_string(cb.as_ptr(), wb, hb);
        let res = if ma.is_null() || mb.is_null() {
            std::ptr::null_mut()
        } else {
            p.c.multiply_matrices(ma, mb)
        };
        let s = snapshot(res, true);
        p.c.free_matrix(res);
        p.c.free_matrix(ma);
        p.c.free_matrix(mb);
        s
    });
    snapshot_is_string_safe(&snap)
}

/// `diff_to_string`, skipped (returning `false`) when the C would overflow its
/// own buffer for this input.
pub fn diff_to_string_if_safe(label: &str, input: &str, w: c_int, h: c_int) -> bool {
    if !init_is_string_safe(input, w, h) {
        return false;
    }
    diff_to_string(label, input, w, h);
    true
}

/// `diff_pipeline`, skipped when the product would overflow the C's buffer.
pub fn diff_pipeline_if_safe(
    label: &str,
    a: &str,
    wa: c_int,
    ha: c_int,
    b: &str,
    wb: c_int,
    hb: c_int,
) -> bool {
    if !product_is_string_safe(a, wa, ha, b, wb, hb) {
        return false;
    }
    diff_pipeline(label, a, wa, ha, b, wb, hb);
    true
}

/// `diff_driver`, skipped when the product would overflow the C's buffer.
pub fn diff_driver_if_safe(
    label: &str,
    wa: c_int,
    ha: c_int,
    a: &str,
    wb: c_int,
    hb: c_int,
    b: &str,
) -> bool {
    if !product_is_string_safe(a, wa, ha, b, wb, hb) {
        return false;
    }
    diff_driver(label, wa, ha, a, wb, hb, b);
    true
}

/// Render a matrix of values the way the C input format expects.
pub fn render(vals: &[Vec<i32>]) -> String {
    let mut s = String::new();
    for row in vals {
        let parts: Vec<String> = row.iter().map(|v| v.to_string()).collect();
        s.push_str(&parts.join(" "));
        s.push('\n');
    }
    s
}

/// `strerror`-free description used only in assertion messages.
pub fn errno_name(e: c_int) -> &'static str {
    match e {
        0 => "OK",
        2 => "ENOENT",
        13 => "EACCES",
        14 => "EFAULT",
        21 => "EISDIR",
        22 => "EINVAL",
        28 => "ENOSPC",
        _ => "other",
    }
}

/// Assert a captured stderr blob equals an expected literal.
pub fn assert_stderr_is(label: &str, got: &[u8], expect: &str) {
    assert_eq!(show(got), expect, "[{label}] stderr text mismatch");
}

pub fn cstr_bytes(c: &CStr) -> &[u8] {
    c.to_bytes()
}
