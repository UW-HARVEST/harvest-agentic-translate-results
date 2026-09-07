//! Shared differential-testing harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and every
//! call goes through `dlsym`, so the Rust `#[no_mangle] extern "C"` export
//! wrappers are exercised exactly as an external C consumer would exercise
//! them. Nothing in this harness calls a Rust function directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::fs::File;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

pub type GotomachFn = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;
pub type OpFn = unsafe extern "C" fn(i32, i32, *mut c_void) -> i32;

/// fd 1 is process-global, so only one capture may be in flight at a time.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C shared library produced by `c_src/build`.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("GOTOMACH_C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}. Build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && cmake .. \
         -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// Locate the Rust `cdylib`. Prefers whichever of release/debug is newer so the
/// test always runs against the most recently built artifact.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("GOTOMACH_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    let cands = [
        base.join("release/libgotomach_lib.so"),
        base.join("debug/libgotomach_lib.so"),
    ];
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for c in cands.iter() {
        if let Ok(md) = std::fs::metadata(c) {
            let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                best = Some((t, c.clone()));
            }
        }
    }
    match best {
        Some((_, p)) => p,
        None => panic!(
            "no Rust cdylib found under {}; run `cargo build --release` first",
            base.display()
        ),
    }
}

pub struct Libs {
    pub c: Library,
    pub r: Library,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

fn load(path: &Path) -> Library {
    unsafe { Library::new(path) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()))
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        Libs {
            c: load(&c_path),
            r: load(&r_path),
            c_path,
            r_path,
        }
    })
}

fn tmp_path() -> PathBuf {
    let n = TMP_SEQ.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!(
        "gotomach-diff-{}-{}.out",
        std::process::id(),
        n
    ))
}

/// Run `f` with fd 1 redirected to a temp file and return `f`'s value plus the
/// bytes it wrote to stdout. `fflush(NULL)` is used on both sides of the
/// redirect so the C library's `puts` output (glibc stdio, possibly fully
/// buffered) lands in the capture file and nowhere else.
pub fn capture<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = tmp_path();
    let file = File::create(&path).expect("create capture file");
    unsafe {
        let _ = std::io::stdout().flush();
        libc::fflush(std::ptr::null_mut());
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
        let out = f();
        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);
        drop(file);
        let bytes = std::fs::read(&path).expect("read capture file");
        let _ = std::fs::remove_file(&path);
        (out, bytes)
    }
}

/// RAII guard that points fd 1 at `/dev/null` for its lifetime. Used by the
/// exhaustive sweeps, which make millions of calls and would otherwise bury the
/// test results in log lines. It takes the same lock as `capture`, so the two
/// can never interleave.
pub struct Silence {
    _guard: std::sync::MutexGuard<'static, ()>,
    saved: i32,
    devnull: File,
}

impl Silence {
    pub fn new() -> Self {
        let guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let devnull = File::create("/dev/null").expect("open /dev/null");
        unsafe {
            let _ = std::io::stdout().flush();
            libc::fflush(std::ptr::null_mut());
            let saved = libc::dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(libc::dup2(devnull.as_raw_fd(), 1) >= 0, "dup2 failed");
            Silence {
                _guard: guard,
                saved,
                devnull,
            }
        }
    }
}

impl Drop for Silence {
    fn drop(&mut self) {
        unsafe {
            libc::fflush(std::ptr::null_mut());
            libc::dup2(self.saved, 1);
            libc::close(self.saved);
        }
        let _ = &self.devnull;
    }
}

fn goto_sym<'a>(lib: &'a Library) -> Symbol<'a, GotomachFn> {
    unsafe { lib.get(b"gotomach\0") }.expect("dlsym gotomach")
}

fn op_sym<'a>(lib: &'a Library, name: &str) -> Symbol<'a, OpFn> {
    let mut n = name.as_bytes().to_vec();
    n.push(0);
    unsafe { lib.get(&n) }.unwrap_or_else(|e| panic!("dlsym {name}: {e}"))
}

/// One `gotomach` call against one library: returns (retval, stdout bytes).
fn call_gotomach(lib: &Library, it: i32, seed: i32, mode: i32, th: i32) -> (i32, Vec<u8>) {
    let f = goto_sym(lib);
    capture(|| unsafe { f(it, seed, mode, th) })
}

fn call_op(lib: &Library, name: &str, v: i32, p: i32, ctx: *mut c_void) -> (i32, Vec<u8>) {
    let f = op_sym(lib, name);
    capture(|| unsafe { f(v, p, ctx) })
}

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).replace('\n', "\\n")
}

/// Differential assertion for `gotomach`: return value AND stdout must match
/// byte-for-byte.
#[track_caller]
pub fn diff_gotomach(row: &str, it: i32, seed: i32, mode: i32, th: i32) -> i32 {
    let l = libs();
    let (cr, co) = call_gotomach(&l.c, it, seed, mode, th);
    let (rr, ro) = call_gotomach(&l.r, it, seed, mode, th);
    assert_eq!(
        cr, rr,
        "[{row}] return value mismatch for gotomach({it}, {seed}, {mode}, {th}): C={cr} Rust={rr}"
    );
    assert_eq!(
        co,
        ro,
        "[{row}] stdout mismatch for gotomach({it}, {seed}, {mode}, {th}):\n  C   = {}\n  Rust= {}",
        show(&co),
        show(&ro)
    );
    cr
}

/// Differential assertion for one of the three operation callbacks.
#[track_caller]
pub fn diff_op(row: &str, name: &str, v: i32, p: i32, ctx: *mut c_void) -> i32 {
    let l = libs();
    let (cr, co) = call_op(&l.c, name, v, p, ctx);
    let (rr, ro) = call_op(&l.r, name, v, p, ctx);
    assert_eq!(
        cr, rr,
        "[{row}] return value mismatch for {name}({v}, {p}, {ctx:p}): C={cr} Rust={rr}"
    );
    assert_eq!(
        co,
        ro,
        "[{row}] stdout mismatch for {name}({v}, {p}, {ctx:p}):\n  C   = {}\n  Rust= {}",
        show(&co),
        show(&ro)
    );
    cr
}

/// Deterministic xorshift64* PRNG so every property-style row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `[lo, hi]` inclusive, for `lo <= hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}
