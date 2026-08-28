// Shared differential-test harness.
//
// Both the C library (built by c_src/CMakeLists.txt) and the Rust library
// (crate-type = ["cdylib"]) are loaded with `libloading` and driven ONLY through
// their exported C symbols, so the `#[unsafe(no_mangle)] extern "C"` wrappers are
// part of what is under test.
//
// The library under test writes to `stdout`/`stderr` through the *C* stdio
// functions, so the comparison captures those at the file-descriptor level
// (dup2), flushes, and compares the raw bytes.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_long, c_void, CString};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits we need directly
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fclose(stream: *mut c_void) -> c_int;
    fn ferror(stream: *mut c_void) -> c_int;
    fn feof(stream: *mut c_void) -> c_int;
    fn ftell(stream: *mut c_void) -> c_long;
    fn fileno(stream: *mut c_void) -> c_int;
    fn getuid() -> u32;
}

pub fn is_root() -> bool {
    unsafe { getuid() == 0 }
}

/// `fclose` a stream handed back by `open_with_cleanup` (the caller owns it).
pub unsafe fn close_stream(fp: *mut c_void) -> c_int {
    if fp.is_null() {
        0
    } else {
        fclose(fp)
    }
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub type FnDriver = unsafe extern "C" fn(c_int, *const c_char) -> c_int;
pub type FnForward = unsafe extern "C" fn(c_int) -> c_int;
pub type FnOpen = unsafe extern "C" fn(*const c_char) -> *mut c_void;

/// Names exported by the C `.so` that the Rust `.so` must also export.
pub const EXPORTED_SYMBOLS: &[&str] = &["driver", "forward_goto_example", "open_with_cleanup"];

pub struct Libs {
    pub c: Library,
    pub rust: Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let p = repo_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Prefer the release artifact (the shipped one); fall back to debug.
    for prof in ["release", "debug"] {
        let p = manifest.join("target").join(prof).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!("Rust cdylib not found; run `cargo build --release` first");
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            let c = Library::new(&c_path).expect("dlopen C .so");
            let rust = Library::new(&rust_path).expect("dlopen Rust .so");
            Libs {
                c,
                rust,
                c_path,
                rust_path,
            }
        }
    })
}

macro_rules! getsym {
    ($lib:expr, $name:literal, $ty:ty) => {{
        let s: Symbol<$ty> = unsafe {
            $lib.get(concat!($name, "\0").as_bytes())
                .unwrap_or_else(|e| panic!("missing symbol `{}`: {e}", $name))
        };
        *s
    }};
}

pub fn c_driver() -> FnDriver {
    getsym!(libs().c, "driver", FnDriver)
}
pub fn r_driver() -> FnDriver {
    getsym!(libs().rust, "driver", FnDriver)
}
pub fn c_forward() -> FnForward {
    getsym!(libs().c, "forward_goto_example", FnForward)
}
pub fn r_forward() -> FnForward {
    getsym!(libs().rust, "forward_goto_example", FnForward)
}
pub fn c_open() -> FnOpen {
    getsym!(libs().c, "open_with_cleanup", FnOpen)
}
pub fn r_open() -> FnOpen {
    getsym!(libs().rust, "open_with_cleanup", FnOpen)
}

// ---------------------------------------------------------------------------
// fd-level stdout/stderr capture (serialized: fd 1/2 are process-global)
// ---------------------------------------------------------------------------

struct CaptureState {
    out: File,
    err: File,
}

static CAPTURE: OnceLock<Mutex<CaptureState>> = OnceLock::new();

fn capture_state() -> MutexGuard<'static, CaptureState> {
    let m = CAPTURE.get_or_init(|| {
        let dir = scratch_dir();
        let mk = |n: &str| {
            fs::OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(true)
                .open(dir.join(n))
                .expect("create capture file")
        };
        Mutex::new(CaptureState {
            out: mk("capture.out"),
            err: mk("capture.err"),
        })
    });
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn reset(f: &mut File) {
    f.set_len(0).expect("truncate capture file");
    f.seek(SeekFrom::Start(0)).expect("rewind capture file");
}

fn drain(f: &mut File) -> Vec<u8> {
    f.seek(SeekFrom::Start(0)).expect("rewind capture file");
    let mut v = Vec::new();
    f.read_to_end(&mut v).expect("read capture file");
    reset(f);
    v
}

/// Run `f` with fd 1 and fd 2 redirected into scratch files; return its value
/// together with the exact bytes it wrote to stdout and stderr.
pub fn capture<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>, Vec<u8>) {
    let mut st = capture_state();

    // fd 1 / fd 2 are process-global, so nothing else may write to them while
    // the redirection is installed. Holding Rust's stdout/stderr locks keeps
    // libtest's own progress output (and any `print!`) out of the window; the
    // locks are re-entrant, so a panic from inside `f` still reports normally.
    let _lock_out = std::io::stdout().lock();
    let _lock_err = std::io::stderr().lock();

    // Make sure nothing already buffered leaks into this capture.
    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    reset(&mut st.out);
    reset(&mut st.err);

    let (fo, fe) = (st.out.as_raw_fd(), st.err.as_raw_fd());
    let (s1, s2) = unsafe { (dup(1), dup(2)) };
    assert!(s1 >= 0 && s2 >= 0, "dup() failed");
    unsafe {
        assert!(dup2(fo, 1) >= 0, "dup2 stdout");
        assert!(dup2(fe, 2) >= 0, "dup2 stderr");
    }

    let r = f();

    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(s1, 1) >= 0, "restore stdout");
        assert!(dup2(s2, 2) >= 0, "restore stderr");
        close(s1);
        close(s2);
    }

    let out = drain(&mut st.out);
    let err = drain(&mut st.err);
    (r, out, err)
}

// ---------------------------------------------------------------------------
// Observable outcome of one call
// ---------------------------------------------------------------------------

/// Everything observable about a single call, for byte-exact comparison.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Obs {
    /// `int` return value, or for `FILE*` returns: 0 = NULL, 1 = non-NULL.
    pub ret: i64,
    /// For `FILE*` returns only: (ftell, feof != 0, ferror != 0, fd is valid).
    pub stream: Option<(i64, bool, bool, bool)>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// Change in the number of open file descriptors across the call.
    ///
    /// This is what makes the `fclose(fp)` inside `open_with_cleanup`'s cleanup
    /// block observable: dropping it changes nothing about the return value or
    /// the printed bytes, but leaks one descriptor per call.
    pub fd_delta: i64,
}

impl Obs {
    pub fn describe(&self) -> String {
        format!(
            "ret={} stream={:?} fd_delta={}\n  stdout({} bytes)={}\n  stderr({} bytes)={}",
            self.ret,
            self.stream,
            self.fd_delta,
            self.stdout.len(),
            esc(&self.stdout),
            self.stderr.len(),
            esc(&self.stderr)
        )
    }
}

/// Force all lazy one-time initialisation (dlopen of both `.so`s, creation of
/// the two capture scratch files) to happen BEFORE any descriptor baseline is
/// taken — otherwise the first measured call appears to "leak" the capture files.
pub fn warm() {
    static W: OnceLock<()> = OnceLock::new();
    W.get_or_init(|| {
        let _ = libs();
        drop(capture_state());
    });
}

/// Number of descriptors currently open in this process (0 if /proc is absent).
pub fn fd_count() -> i64 {
    match fs::read_dir("/proc/self/fd") {
        Ok(d) => d.count() as i64,
        Err(_) => 0,
    }
}

/// Compact, actionable byte-exact comparison. Panics on the first divergence
/// with the offending offset and a small window around it (dumping two full
/// multi-megabyte buffers would be useless).
#[track_caller]
pub fn compare(label: &str, c: &Obs, r: &Obs) {
    if c == r {
        return;
    }
    let mut msg = format!("DIVERGENCE in {label}\n");
    if c.ret != r.ret {
        msg += &format!("  return value: C={} RUST={}\n", c.ret, r.ret);
    }
    if c.stream != r.stream {
        msg += &format!(
            "  returned FILE* state (ftell, feof, ferror, fileno_ok): C={:?} RUST={:?}\n",
            c.stream, r.stream
        );
    }
    if c.fd_delta != r.fd_delta {
        msg += &format!(
            "  open-fd delta: C={} RUST={}  (a non-zero Rust delta means a stream \
             was not fclose()d)\n",
            c.fd_delta, r.fd_delta
        );
    }
    for (name, a, b) in [
        ("stdout", &c.stdout, &r.stdout),
        ("stderr", &c.stderr, &r.stderr),
    ] {
        if a == b {
            continue;
        }
        let at = a
            .iter()
            .zip(b.iter())
            .position(|(x, y)| x != y)
            .unwrap_or(a.len().min(b.len()));
        let lo = at.saturating_sub(40);
        msg += &format!(
            "  {name}: C is {} bytes, RUST is {} bytes; first difference at byte {at}\n\
             \x20   C   [{lo}..]={}\n\
             \x20   RUST[{lo}..]={}\n",
            a.len(),
            b.len(),
            esc(&a[lo..a.len().min(lo + 120)]),
            esc(&b[lo..b.len().min(lo + 120)]),
        );
    }
    panic!("{msg}");
}

pub fn esc(b: &[u8]) -> String {
    const MAX: usize = 400;
    let mut s = String::new();
    for &c in b.iter().take(MAX) {
        match c {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\0' => s.push_str("\\0"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    if b.len() > MAX {
        s.push_str(&format!("... (+{} bytes)", b.len() - MAX));
    }
    format!("\"{s}\"")
}

// --- forward_goto_example ---------------------------------------------------

fn obs_forward(f: FnForward, x: c_int) -> Obs {
    warm();
    let before = fd_count();
    let (ret, stdout, stderr) = capture(|| unsafe { f(x) });
    Obs {
        ret: ret as i64,
        stream: None,
        stdout,
        stderr,
        fd_delta: fd_count() - before,
    }
}

#[track_caller]
pub fn diff_forward(x: c_int, ctx: &str) {
    let a = obs_forward(c_forward(), x);
    let b = obs_forward(r_forward(), x);
    compare(&format!("forward_goto_example({x})  [{ctx}]"), &a, &b);
}

// --- open_with_cleanup -----------------------------------------------------

fn obs_open(f: FnOpen, name: Option<&[u8]>) -> Obs {
    let cs = name.map(|n| CString::new(n).expect("filename must not contain NUL"));
    let p = cs.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());

    warm();
    let before = fd_count();
    let ((ret, stream), stdout, stderr) = capture(|| unsafe {
        let fp = f(p);
        if fp.is_null() {
            (0i64, None)
        } else {
            let st = (
                ftell(fp) as i64,
                feof(fp) != 0,
                ferror(fp) != 0,
                fileno(fp) >= 0,
            );
            // The caller owns the returned stream (this is what driver() does).
            fclose(fp);
            (1i64, Some(st))
        }
    });

    Obs {
        ret,
        stream,
        stdout,
        stderr,
        fd_delta: fd_count() - before,
    }
}

#[track_caller]
pub fn diff_open(name: Option<&[u8]>, ctx: &str) {
    let a = obs_open(c_open(), name);
    let b = obs_open(r_open(), name);
    compare(
        &format!(
            "open_with_cleanup({})  [{ctx}]",
            name.map_or("NULL".to_string(), esc)
        ),
        &a,
        &b,
    );
}

#[track_caller]
pub fn diff_open_path(p: &Path, ctx: &str) {
    diff_open(Some(p.as_os_str().as_bytes()), ctx)
}

// --- driver ----------------------------------------------------------------

fn obs_driver(f: FnDriver, num: c_int, name: Option<&[u8]>) -> Obs {
    let cs = name.map(|n| CString::new(n).expect("filename must not contain NUL"));
    let p = cs.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
    warm();
    let before = fd_count();
    let (ret, stdout, stderr) = capture(|| unsafe { f(num, p) });
    Obs {
        ret: ret as i64,
        stream: None,
        stdout,
        stderr,
        fd_delta: fd_count() - before,
    }
}

#[track_caller]
pub fn diff_driver(num: c_int, name: Option<&[u8]>, ctx: &str) {
    let a = obs_driver(c_driver(), num, name);
    let b = obs_driver(r_driver(), num, name);
    compare(
        &format!(
            "driver({num}, {})  [{ctx}]",
            name.map_or("NULL".to_string(), esc)
        ),
        &a,
        &b,
    );
}

#[track_caller]
pub fn diff_driver_path(num: c_int, p: &Path, ctx: &str) {
    diff_driver(num, Some(p.as_os_str().as_bytes()), ctx)
}

// ---------------------------------------------------------------------------
// Scratch files
// ---------------------------------------------------------------------------

pub fn scratch_dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let base = std::env::var_os("TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        let d = base.join(format!("goto-difftest-{}", std::process::id()));
        fs::create_dir_all(&d).expect("create scratch dir");
        d
    })
    .clone()
}

/// Write `bytes` to a uniquely named scratch file and return its path.
pub fn make_file(tag: &str, bytes: &[u8]) -> PathBuf {
    static N: Mutex<u64> = Mutex::new(0);
    let n = {
        let mut g = N.lock().unwrap_or_else(|e| e.into_inner());
        *g += 1;
        *g
    };
    let p = scratch_dir().join(format!("{tag}-{n}.dat"));
    let mut f = File::create(&p).expect("create scratch file");
    f.write_all(bytes).expect("write scratch file");
    f.flush().expect("flush scratch file");
    drop(f);
    // Make sure the mode is deterministic for both halves of the comparison.
    fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).expect("chmod scratch file");
    p
}

/// Like [`make_file`] but always uses the same path for a given `tag`
/// (overwritten in place) — keeps the scratch directory small when a row runs
/// hundreds of randomized iterations.
pub fn put_file(tag: &str, bytes: &[u8]) -> PathBuf {
    let p = scratch_dir().join(format!("{tag}.dat"));
    let mut f = File::create(&p).expect("create scratch file");
    f.write_all(bytes).expect("write scratch file");
    f.flush().expect("flush scratch file");
    drop(f);
    fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).expect("chmod scratch file");
    p
}

pub fn make_dir(tag: &str) -> PathBuf {
    let p = scratch_dir().join(format!("{tag}-dir"));
    fs::create_dir_all(&p).expect("create scratch subdir");
    p
}

pub fn missing_path(tag: &str) -> PathBuf {
    let p = scratch_dir().join(format!("{tag}-does-not-exist"));
    let _ = fs::remove_file(&p);
    p
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5DEECE66D;

pub struct Rng(u64);

impl Rng {
    pub fn new(salt: u64) -> Self {
        let mut s = SEED ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        if s == 0 {
            s = 0xDEAD_BEEF_CAFE_BABE;
        }
        Rng(s)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn i32_any(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `[lo, hi]` (inclusive), `lo <= hi`.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as i64
    }
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        self.range_i64(lo as i64, hi as i64) as usize
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    /// Printable-ASCII (no `\n`, no `\0`) blob.
    pub fn text(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| {
                let c = 0x20u8 + (self.byte() % 95);
                c
            })
            .collect()
    }
    /// Arbitrary bytes, including `\0`, `\n`, `%` and high-bit values.
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.byte()).collect()
    }
}

/// Build a text file body of `lines` lines with random lengths in `0..=maxlen`.
pub fn random_lines(rng: &mut Rng, lines: usize, maxlen: usize, trailing_nl: bool) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..lines {
        let n = rng.range(0, maxlen);
        v.extend_from_slice(&rng.text(n));
        if i + 1 < lines || trailing_nl {
            v.push(b'\n');
        }
    }
    v
}
