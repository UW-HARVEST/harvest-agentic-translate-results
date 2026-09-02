//! Shared differential-testing harness.
//!
//! Both the original C shared object (`c_src/build/libdriver.so`) and the
//! translated Rust shared object (`translation/target/<profile>/libdriver.so`)
//! are loaded with `libloading` and driven **only** through their exported
//! symbols. The Rust crate is never linked directly, so the `#[no_mangle]`
//! `extern "C"` wrappers are part of what is under test.
//!
//! Because both objects bind to the same `libc.so.6` at run time they share one
//! set of `FILE` objects for `stdout`/`stderr`, which is what makes
//! byte-for-byte stream comparison inside a single test process valid.

#![allow(dead_code)]

use std::ffi::{CString, c_char, c_int, c_long, c_void};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits needed by the harness itself (fd juggling + inspecting a FILE*)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open output stream in the process.
    fn fflush(stream: *mut c_void) -> c_int;
    fn fclose(stream: *mut c_void) -> c_int;
    fn ftell(stream: *mut c_void) -> c_long;
    fn feof(stream: *mut c_void) -> c_int;
    fn ferror(stream: *mut c_void) -> c_int;
    fn fileno(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// The exported ABI of the library under test
// ---------------------------------------------------------------------------

/// Raw function pointers resolved out of one shared object via `dlsym`.
pub struct Api {
    pub which: &'static str,
    pub forward_goto_example: extern "C" fn(c_int) -> c_int,
    pub open_with_cleanup: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    pub driver: unsafe extern "C" fn(c_int, *const c_char) -> c_int,
}

impl Api {
    fn load(which: &'static str, path: &Path) -> Api {
        // Leaked on purpose: the resolved function pointers must stay valid for
        // the whole test binary's lifetime.
        let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("could not dlopen {}: {e}", path.display()))
        }));

        macro_rules! sym {
            ($name:literal, $ty:ty) => {{
                let s: libloading::Symbol<$ty> = unsafe {
                    lib.get(concat!($name, "\0").as_bytes()).unwrap_or_else(|e| {
                        panic!("{} is missing symbol `{}`: {e}", path.display(), $name)
                    })
                };
                *s
            }};
        }

        Api {
            which,
            forward_goto_example: sym!("forward_goto_example", extern "C" fn(c_int) -> c_int),
            open_with_cleanup: sym!(
                "open_with_cleanup",
                unsafe extern "C" fn(*const c_char) -> *mut c_void
            ),
            driver: sym!("driver", unsafe extern "C" fn(c_int, *const c_char) -> c_int),
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("crate has a parent directory")
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir();
    let release = root.join("target/release/libdriver.so");
    let debug = root.join("target/debug/libdriver.so");
    // Prefer whichever exists; release matches the shipped `panic = "abort"`
    // profile, so try it first.
    if release.exists() {
        release
    } else if debug.exists() {
        debug
    } else {
        panic!(
            "neither {} nor {} exists - run `cargo build --release` first",
            release.display(),
            debug.display()
        )
    }
}

/// The C reference implementation.
pub fn c_api() -> &'static Api {
    static C: OnceLock<Api> = OnceLock::new();
    C.get_or_init(|| Api::load("C", &c_so_path()))
}

/// The Rust translation, loaded as an external caller would.
pub fn rust_api() -> &'static Api {
    static R: OnceLock<Api> = OnceLock::new();
    R.get_or_init(|| Api::load("Rust", &rust_so_path()))
}

/// Both, so a test can iterate `[c, rust]` uniformly.
pub fn both() -> (&'static Api, &'static Api) {
    (c_api(), rust_api())
}

// ---------------------------------------------------------------------------
// stdout/stderr capture
// ---------------------------------------------------------------------------

/// Everything one call wrote, plus what it returned.
#[derive(Clone, PartialEq, Eq)]
pub struct Captured {
    pub out: Vec<u8>,
    pub err: Vec<u8>,
}

impl std::fmt::Debug for Captured {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Captured")
            .field("out", &Show(&self.out))
            .field("err", &Show(&self.err))
            .finish()
    }
}

/// Byte-slice formatter that stays readable for binary payloads.
struct Show<'a>(&'a [u8]);

impl std::fmt::Debug for Show<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.0.len();
        let head = &self.0[..n.min(400)];
        write!(f, "{} bytes: {:?}", n, String::from_utf8_lossy(head))?;
        if n > head.len() {
            write!(f, "…")?;
        }
        Ok(())
    }
}

/// fd redirection is process-global, so captures must not overlap.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn unique_tmp(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "gotodiff-{}-{}-{}-{}",
        std::process::id(),
        tag,
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ))
}

fn slurp(path: &Path) -> Vec<u8> {
    let mut v = Vec::new();
    std::fs::File::open(path)
        .expect("capture file exists")
        .read_to_end(&mut v)
        .expect("capture file readable");
    let _ = std::fs::remove_file(path);
    v
}

/// Run `f` with fds 1 and 2 redirected into separate files and return both
/// byte streams alongside `f`'s value.
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Captured) {
    let _guard = capture_lock();

    let out_path = unique_tmp("out");
    let err_path = unique_tmp("err");
    let out_file = std::fs::File::create(&out_path).expect("create stdout capture file");
    let err_file = std::fs::File::create(&err_path).expect("create stderr capture file");

    unsafe {
        // Drain anything already buffered so it is not attributed to `f`.
        fflush(std::ptr::null_mut());
        let saved_out = dup(1);
        let saved_err = dup(2);
        assert!(saved_out >= 0 && saved_err >= 0, "dup failed");
        assert!(dup2(out_file.as_raw_fd(), 1) >= 0, "dup2 stdout failed");
        assert!(dup2(err_file.as_raw_fd(), 2) >= 0, "dup2 stderr failed");

        let value = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved_out, 1) >= 0, "restore stdout failed");
        assert!(dup2(saved_err, 2) >= 0, "restore stderr failed");
        close(saved_out);
        close(saved_err);

        drop(out_file);
        drop(err_file);
        (value, Captured { out: slurp(&out_path), err: slurp(&err_path) })
    }
}

/// Like [`capture`] but sends fd 1 **and** fd 2 to the same file, so the
/// relative ordering of line-buffered stdout and unbuffered stderr writes is
/// observable (`Captured::err` is left empty).
pub fn capture_merged<R>(f: impl FnOnce() -> R) -> (R, Captured) {
    let _guard = capture_lock();

    let path = unique_tmp("merged");
    let file = std::fs::File::create(&path).expect("create merged capture file");

    unsafe {
        fflush(std::ptr::null_mut());
        let saved_out = dup(1);
        let saved_err = dup(2);
        assert!(saved_out >= 0 && saved_err >= 0, "dup failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 stdout failed");
        assert!(dup2(file.as_raw_fd(), 2) >= 0, "dup2 stderr failed");

        let value = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved_out, 1) >= 0, "restore stdout failed");
        assert!(dup2(saved_err, 2) >= 0, "restore stderr failed");
        close(saved_out);
        close(saved_err);

        drop(file);
        (value, Captured { out: slurp(&path), err: Vec::new() })
    }
}

// ---------------------------------------------------------------------------
// Differential drivers, one per exported entry point
// ---------------------------------------------------------------------------

fn diff_streams(what: &str, c: &Captured, r: &Captured) {
    assert_eq!(
        c.out, r.out,
        "{what}: stdout differs\n  C   : {:?}\n  Rust: {:?}",
        Show(&c.out),
        Show(&r.out)
    );
    assert_eq!(
        c.err, r.err,
        "{what}: stderr differs\n  C   : {:?}\n  Rust: {:?}",
        Show(&c.err),
        Show(&r.err)
    );
}

/// Call `forward_goto_example(x)` in both objects; assert return value and both
/// streams match byte-for-byte. Returns the shared value.
pub fn diff_forward(x: c_int) -> c_int {
    let (c, r) = both();
    let (cv, cc) = capture(|| (c.forward_goto_example)(x));
    let (rv, rc) = capture(|| (r.forward_goto_example)(x));
    assert_eq!(cv, rv, "forward_goto_example({x}): return value differs");
    diff_streams(&format!("forward_goto_example({x})"), &cc, &rc);
    cv
}

/// Observable state of the `FILE*` that `open_with_cleanup` hands back.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct HandleState {
    pub null: bool,
    pub tell: c_long,
    pub eof_set: bool,
    pub error_set: bool,
    pub fd_valid: bool,
}

fn inspect_and_close(fp: *mut c_void) -> HandleState {
    if fp.is_null() {
        return HandleState { null: true, tell: 0, eof_set: false, error_set: false, fd_valid: false };
    }
    unsafe {
        let st = HandleState {
            null: false,
            tell: ftell(fp),
            eof_set: feof(fp) != 0,
            error_set: ferror(fp) != 0,
            fd_valid: fileno(fp) >= 0,
        };
        // The C returns the handle still open; the caller owns closing it.
        fclose(fp);
        st
    }
}

/// Call `open_with_cleanup(filename)` in both objects. Compares NULL-ness, the
/// returned handle's observable state, and both streams.
pub fn diff_open(filename: Option<&[u8]>) -> HandleState {
    let owned = filename.map(|f| CString::new(f).expect("filename has no interior NUL"));
    let ptr = owned.as_ref().map_or(std::ptr::null(), |s| s.as_ptr());
    let label = format!(
        "open_with_cleanup({})",
        filename.map_or("NULL".to_string(), |f| format!("{:?}", String::from_utf8_lossy(f)))
    );

    let (c, r) = both();
    let (cfp, cc) = capture(|| unsafe { (c.open_with_cleanup)(ptr) });
    let cst = inspect_and_close(cfp);
    let (rfp, rc) = capture(|| unsafe { (r.open_with_cleanup)(ptr) });
    let rst = inspect_and_close(rfp);

    assert_eq!(cst.null, rst.null, "{label}: NULL-ness of result differs");
    assert_eq!(cst, rst, "{label}: returned FILE* state differs");
    diff_streams(&label, &cc, &rc);
    cst
}

/// Call `driver(num, filename)` in both objects; compare the exit code and both
/// streams.
pub fn diff_driver(num: c_int, filename: Option<&[u8]>) -> c_int {
    let owned = filename.map(|f| CString::new(f).expect("filename has no interior NUL"));
    let ptr = owned.as_ref().map_or(std::ptr::null(), |s| s.as_ptr());
    let label = format!(
        "driver({num}, {})",
        filename.map_or("NULL".to_string(), |f| format!("{:?}", String::from_utf8_lossy(f)))
    );

    let (c, r) = both();
    let (cv, cc) = capture(|| unsafe { (c.driver)(num, ptr) });
    let (rv, rc) = capture(|| unsafe { (r.driver)(num, ptr) });
    assert_eq!(cv, rv, "{label}: return value differs");
    diff_streams(&label, &cc, &rc);
    cv
}

// ---------------------------------------------------------------------------
// Temp-file scaffolding for the file-shaped inputs
// ---------------------------------------------------------------------------

/// A temp file (or directory) that removes itself on drop.
pub struct TempPath {
    pub path: PathBuf,
    dir: bool,
}

impl TempPath {
    /// Create a regular file with exactly `content` in it.
    pub fn file(content: &[u8]) -> TempPath {
        Self::file_named("f", content)
    }

    /// Same, but with control over the (sanitised) name suffix, so filename
    /// shape can be varied.
    pub fn file_named(tag: &str, content: &[u8]) -> TempPath {
        let path = unique_tmp(tag);
        std::fs::write(&path, content).expect("write temp input file");
        TempPath { path, dir: false }
    }

    /// Create a directory — `fopen` succeeds on Linux but `fgets` fails, which
    /// is how the `ferror` branch is reached.
    pub fn dir() -> TempPath {
        let path = unique_tmp("dir");
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempPath { path, dir: true }
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.path.as_os_str().as_encoded_bytes().to_vec()
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        let _ = if self.dir {
            std::fs::remove_dir_all(&self.path)
        } else {
            std::fs::remove_file(&self.path)
        };
    }
}

/// A path that is guaranteed not to exist.
pub fn missing_path() -> PathBuf {
    let p = unique_tmp("does-not-exist");
    let _ = std::fs::remove_file(&p);
    p
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed keeps failures reproducible
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_600F_C0DE_1234;

pub struct Rng(u64);

impl Rng {
    pub fn new(stream: u64) -> Rng {
        Rng(SEED ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15))
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

    /// Uniform in `0..n` (`n > 0`).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    /// Uniform in `lo..=hi` over the whole `i32` range.
    pub fn in_range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
}
