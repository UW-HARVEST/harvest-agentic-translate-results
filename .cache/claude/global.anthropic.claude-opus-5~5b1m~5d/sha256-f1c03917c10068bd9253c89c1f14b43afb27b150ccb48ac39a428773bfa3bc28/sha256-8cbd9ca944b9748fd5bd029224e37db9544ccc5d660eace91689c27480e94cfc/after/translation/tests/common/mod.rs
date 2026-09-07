//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and every
//! call goes through the exported symbols, so the `#[no_mangle]`/`extern "C"`
//! wrappers are part of what is under test.  Nothing in the Rust crate is ever
//! called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// libc bits used to capture the C stdio streams that BOTH libraries write to.
// ---------------------------------------------------------------------------
extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fclose(stream: *mut c_void) -> c_int;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

/// Which of the two implementations to exercise.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

impl Impl {
    pub fn name(self) -> &'static str {
        match self {
            Impl::C => "C",
            Impl::Rust => "Rust",
        }
    }
}

pub const BOTH: [Impl; 2] = [Impl::C, Impl::Rust];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    // Allow the runner to pin a specific artifact so that BOTH the `dev` and the
    // `release` cdylib (which differ in optimisation level and in
    // `panic = "abort"`) can be verified against the same C library.
    if let Some(p) = std::env::var_os("CTORUST_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "CTORUST_RUST_SO={p:?} does not exist");
        return p;
    }
    // Otherwise pick the most recently built artifact so the test always
    // exercises fresh code.
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for profile in ["release", "debug"] {
        let p = manifest_dir().join("target").join(profile).join("libdriver.so");
        if let Ok(md) = std::fs::metadata(&p) {
            let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                best = Some((t, p));
            }
        }
    }
    best.map(|(_, p)| p).unwrap_or_else(|| {
        panic!(
            "Rust shared library not found under target/{{debug,release}}/libdriver.so. \
             Build it with `cargo build` and/or `cargo build --release`."
        )
    })
}

struct Libs {
    c: Library,
    rust: Library,
}

// The loaded libraries live for the whole test process; `Library` is Send+Sync.
static LIBS: OnceLock<Libs> = OnceLock::new();

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("dlopen C libdriver.so");
        let rust = Library::new(rust_so_path()).expect("dlopen Rust libdriver.so");
        Libs { c, rust }
    })
}

fn lib(which: Impl) -> &'static Library {
    match which {
        Impl::C => &libs().c,
        Impl::Rust => &libs().rust,
    }
}

// Exported signatures, exactly as in include/goto.h and src/goto.c.
type FnDriver = unsafe extern "C" fn(c_int, *const c_char) -> c_int;
type FnForward = unsafe extern "C" fn(c_int) -> c_int;
type FnOpen = unsafe extern "C" fn(*const c_char) -> *mut c_void;

fn sym<T>(which: Impl, name: &str) -> Symbol<'static, T> {
    unsafe {
        lib(which)
            .get::<T>(name.as_bytes())
            .unwrap_or_else(|e| panic!("{} .so is missing symbol `{name}`: {e}", which.name()))
    }
}

// ---------------------------------------------------------------------------
// stdout / stderr capture
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub struct Output {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Output {{ stdout: {:?}, stderr: {:?} }}",
            String::from_utf8_lossy(&self.stdout),
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

fn unique_tmp(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "ctorust_goto_{}_{}_{}_{}",
        tag,
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ))
}

struct Redirect {
    fd: c_int,
    saved: c_int,
    tmp_fd: c_int,
    path: PathBuf,
}

impl Redirect {
    /// Point `fd` at a fresh temp file, remembering the old target.
    fn start(fd: c_int, tag: &str) -> Redirect {
        let path = unique_tmp(tag);
        let cpath = CString::new(path.to_str().unwrap()).unwrap();
        let tmp_fd = unsafe { open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int) };
        assert!(tmp_fd >= 0, "open({path:?}) failed");
        let saved = unsafe { dup(fd) };
        assert!(saved >= 0, "dup({fd}) failed");
        assert!(unsafe { dup2(tmp_fd, fd) } >= 0, "dup2 onto {fd} failed");
        Redirect { fd, saved, tmp_fd, path }
    }

    /// Reuse an already-open temp file for a second fd (combined capture).
    fn start_shared(fd: c_int, tmp_fd: c_int) -> Redirect {
        let saved = unsafe { dup(fd) };
        assert!(saved >= 0, "dup({fd}) failed");
        assert!(unsafe { dup2(tmp_fd, fd) } >= 0, "dup2 onto {fd} failed");
        Redirect { fd, saved, tmp_fd: -1, path: PathBuf::new() }
    }

    /// Read back what was written.  The fd is restored by `Drop`, so an
    /// unexpected panic inside the captured closure can never leave the test
    /// process with a hijacked stdout.
    fn read_back(&self) -> Vec<u8> {
        if self.tmp_fd < 0 {
            return Vec::new();
        }
        let mut buf = Vec::new();
        std::fs::File::open(&self.path)
            .expect("reopen capture file")
            .read_to_end(&mut buf)
            .expect("read capture file");
        buf
    }
}

impl Drop for Redirect {
    fn drop(&mut self) {
        unsafe {
            dup2(self.saved, self.fd);
            close(self.saved);
            if self.tmp_fd >= 0 {
                close(self.tmp_fd);
            }
        }
        if self.tmp_fd >= 0 {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// fd 1 and fd 2 are process-global, so only one capture may be in flight.
/// `.cargo/config.toml` additionally pins `RUST_TEST_THREADS=1` so that
/// libtest's own progress output can never land inside a capture window.
fn capture_lock() -> std::sync::MutexGuard<'static, ()> {
    static L: std::sync::Mutex<()> = std::sync::Mutex::new(());
    match L.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Drain every buffer that could otherwise be flushed into the capture file:
/// the C stdio streams (used by both libraries) *and* Rust's own line-buffered
/// `stdout`/`stderr`, which still hold libtest's pending `test foo ... ` text.
fn drain_all_streams() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    unsafe { fflush(std::ptr::null_mut()) };
}

/// Run `f` with fds 1 and 2 redirected into separate temp files and return
/// everything the C stdio layer emitted.  `fflush(NULL)` drains every stream in
/// the process, which is what makes this work for output produced inside the
/// dlopen'ed libraries.
fn with_capture<R>(f: impl FnOnce() -> R) -> (R, Output) {
    let _g = capture_lock();
    drain_all_streams();
    let out = Redirect::start(1, "out");
    let err = Redirect::start(2, "err");
    let r = f();
    drain_all_streams();
    let stdout = out.read_back();
    let stderr = err.read_back();
    drop(err);
    drop(out);
    (r, Output { stdout, stderr })
}

/// Same, but fds 1 and 2 share one file so the *interleaving* is observable.
fn with_combined_capture<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let _g = capture_lock();
    drain_all_streams();
    let out = Redirect::start(1, "comb");
    let err = Redirect::start_shared(2, out.tmp_fd);
    let r = f();
    drain_all_streams();
    let bytes = out.read_back();
    drop(err);
    drop(out);
    (r, bytes)
}

// ---------------------------------------------------------------------------
// Typed wrappers around the three exported entry points
// ---------------------------------------------------------------------------

/// `int forward_goto_example(int x)`
pub fn call_forward(which: Impl, x: i32) -> (c_int, Output) {
    let f: Symbol<FnForward> = sym(which, "forward_goto_example");
    with_capture(|| unsafe { f(x) })
}

/// `FILE* open_with_cleanup(const char* filename)`
///
/// Returns `(handle_was_null, output)`.  On success the handle is `fclose`d
/// here (mirroring what `driver` does) so the two implementations leak
/// identically-nothing.
pub fn call_open(which: Impl, filename: Option<&Path>) -> (bool, Output) {
    let cpath = filename.map(|p| CString::new(p.as_os_str().as_encoded_bytes()).unwrap());
    call_open_raw(which, cpath.as_ref().map(|c| c.as_ptr()).unwrap_or(std::ptr::null()))
}

/// Same but with a caller-supplied raw `const char*` (for NULL / non-UTF-8 /
/// empty-string boundary cases).
pub fn call_open_raw(which: Impl, filename: *const c_char) -> (bool, Output) {
    let f: Symbol<FnOpen> = sym(which, "open_with_cleanup");
    let (fp, out) = with_capture(|| unsafe { f(filename) });
    let was_null = fp.is_null();
    if !was_null {
        // The C caller owns the returned stream; prove it is a usable FILE*.
        assert_eq!(unsafe { fclose(fp) }, 0, "{}: fclose of returned FILE* failed", which.name());
    }
    (was_null, out)
}

/// `int driver(int num, const char* filename)`
pub fn call_driver(which: Impl, num: i32, filename: Option<&Path>) -> (c_int, Output) {
    let cpath = filename.map(|p| CString::new(p.as_os_str().as_encoded_bytes()).unwrap());
    call_driver_raw(which, num, cpath.as_ref().map(|c| c.as_ptr()).unwrap_or(std::ptr::null()))
}

pub fn call_driver_raw(which: Impl, num: i32, filename: *const c_char) -> (c_int, Output) {
    let f: Symbol<FnDriver> = sym(which, "driver");
    with_capture(|| unsafe { f(num, filename) })
}

/// `driver` with stdout+stderr merged into one stream, to compare ordering.
pub fn call_driver_combined(which: Impl, num: i32, filename: Option<&Path>) -> (c_int, Vec<u8>) {
    let cpath = filename.map(|p| CString::new(p.as_os_str().as_encoded_bytes()).unwrap());
    let ptr = cpath.as_ref().map(|c| c.as_ptr()).unwrap_or(std::ptr::null());
    let f: Symbol<FnDriver> = sym(which, "driver");
    with_combined_capture(|| unsafe { f(num, ptr) })
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

pub fn assert_same<T: PartialEq + std::fmt::Debug>(
    ctx: &str,
    c: (T, Output),
    rust: (T, Output),
) {
    assert_eq!(c.0, rust.0, "return value differs [{ctx}]");
    assert_eq!(
        c.1.stdout,
        rust.1.stdout,
        "stdout differs [{ctx}]\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.1.stdout),
        String::from_utf8_lossy(&rust.1.stdout)
    );
    assert_eq!(
        c.1.stderr,
        rust.1.stderr,
        "stderr differs [{ctx}]\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.1.stderr),
        String::from_utf8_lossy(&rust.1.stderr)
    );
}

/// Differential check of `forward_goto_example`.
pub fn diff_forward(ctx: &str, x: i32) {
    let c = call_forward(Impl::C, x);
    let r = call_forward(Impl::Rust, x);
    assert_same(&format!("{ctx}: forward_goto_example({x})"), c, r);
}

/// Differential check of `open_with_cleanup`.
pub fn diff_open(ctx: &str, path: Option<&Path>) {
    let c = call_open(Impl::C, path);
    let r = call_open(Impl::Rust, path);
    assert_same(&format!("{ctx}: open_with_cleanup({path:?})"), c, r);
}

pub fn diff_open_raw(ctx: &str, path: *const c_char) {
    let c = call_open_raw(Impl::C, path);
    let r = call_open_raw(Impl::Rust, path);
    assert_same(&format!("{ctx}: open_with_cleanup(raw)"), c, r);
}

/// Differential check of `driver`, including merged-stream ordering.
pub fn diff_driver(ctx: &str, num: i32, path: Option<&Path>) {
    let c = call_driver(Impl::C, num, path);
    let r = call_driver(Impl::Rust, num, path);
    let label = format!("{ctx}: driver({num}, {path:?})");
    assert_same(&label, c, r);

    let (cr, cb) = call_driver_combined(Impl::C, num, path);
    let (rr, rb) = call_driver_combined(Impl::Rust, num, path);
    assert_eq!(cr, rr, "combined-capture return differs [{label}]");
    assert_eq!(
        cb,
        rb,
        "merged stdout+stderr differs [{label}]\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&cb),
        String::from_utf8_lossy(&rb)
    );
}

pub fn diff_driver_raw(ctx: &str, num: i32, path: *const c_char) {
    let c = call_driver_raw(Impl::C, num, path);
    let r = call_driver_raw(Impl::Rust, num, path);
    assert_same(&format!("{ctx}: driver({num}, raw)"), c, r);
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A temp file (or directory) removed on drop.
pub struct Fixture {
    pub path: PathBuf,
    is_dir: bool,
}

impl Fixture {
    pub fn file(tag: &str, contents: &[u8]) -> Fixture {
        let path = unique_tmp(tag);
        std::fs::write(&path, contents).expect("write fixture");
        Fixture { path, is_dir: false }
    }

    pub fn dir(tag: &str) -> Fixture {
        let path = unique_tmp(tag);
        std::fs::create_dir_all(&path).expect("create fixture dir");
        Fixture { path, is_dir: true }
    }

    /// A path guaranteed not to exist.
    pub fn missing(tag: &str) -> PathBuf {
        unique_tmp(tag)
    }

    pub fn chmod(&self, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(mode))
            .expect("chmod fixture");
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o700));
        if self.is_dir {
            let _ = std::fs::remove_dir_all(&self.path);
        } else {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed -> reproducible property tests)
// ---------------------------------------------------------------------------

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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn i32_any(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
}
