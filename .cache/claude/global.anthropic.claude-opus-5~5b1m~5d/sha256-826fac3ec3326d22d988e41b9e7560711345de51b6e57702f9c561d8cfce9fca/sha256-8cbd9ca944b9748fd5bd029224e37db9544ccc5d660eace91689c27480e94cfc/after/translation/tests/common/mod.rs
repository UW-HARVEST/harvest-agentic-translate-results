//! Shared harness for the C-vs-Rust differential tests.
//!
//! BOTH implementations are reached exclusively through `dlopen`/`dlsym` on
//! their respective shared objects — the Rust functions are never called
//! directly, so the `#[no_mangle] extern "C"` wrappers are part of what is
//! under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub type GotomachFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type OpFn = unsafe extern "C" fn(c_int, c_int, *mut c_void) -> c_int;

pub struct Impl {
    pub name: &'static str,
    pub gotomach: GotomachFn,
    pub process_value: OpFn,
    pub double_value: OpFn,
    pub triple_value: OpFn,
    // Keep the handle alive for the whole process lifetime.
    _lib: Library,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        unsafe {
            let gotomach: Symbol<GotomachFn> = lib
                .get(b"gotomach\0")
                .unwrap_or_else(|e| panic!("dlsym(gotomach) in {}: {e}", path.display()));
            let process_value: Symbol<OpFn> = lib
                .get(b"process_value\0")
                .unwrap_or_else(|e| panic!("dlsym(process_value) in {}: {e}", path.display()));
            let double_value: Symbol<OpFn> = lib
                .get(b"double_value\0")
                .unwrap_or_else(|e| panic!("dlsym(double_value) in {}: {e}", path.display()));
            let triple_value: Symbol<OpFn> = lib
                .get(b"triple_value\0")
                .unwrap_or_else(|e| panic!("dlsym(triple_value) in {}: {e}", path.display()));
            let (g, p, d, t) = (*gotomach, *process_value, *double_value, *triple_value);
            Impl {
                name,
                gotomach: g,
                process_value: p,
                double_value: d,
                triple_value: t,
                _lib: lib,
            }
        }
    }

    /// Selects the operation the given `mode` makes `gotomach` use, so the
    /// low-level helper rows can be driven through the very same
    /// `operation_fn` indirection the library uses internally.
    pub fn op_for_mode(&self, mode: c_int) -> OpFn {
        match mode {
            1 => self.double_value,
            2 => self.triple_value,
            _ => self.process_value,
        }
    }

    pub fn op_by_name(&self, name: &str) -> OpFn {
        match name {
            "process_value" => self.process_value,
            "double_value" => self.double_value,
            "triple_value" => self.triple_value
            ,
            other => panic!("unknown op {other}"),
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    // Allows pointing the suite at a differently-optimised C build (e.g. an
    // out-of-source -O2 build) without touching c_src/.
    if let Some(p) = std::env::var_os("GOTOMACH_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "GOTOMACH_C_SO={} is not a file", p.display());
        return p;
    }
    let dir = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        dir.display(),
        found
    );
    found.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    // .../target/<profile>/deps/<test-exe>  ->  .../target/<profile>/
    let exe = std::env::current_exe().expect("current_exe");
    let mut candidates = Vec::new();
    if let Some(profile_dir) = exe.parent().and_then(|p| p.parent()) {
        candidates.push(profile_dir.join("libgotomach_lib.so"));
    }
    let target = manifest_dir().join("target");
    candidates.push(target.join("debug/libgotomach_lib.so"));
    candidates.push(target.join("release/libgotomach_lib.so"));
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "libgotomach_lib.so not found (looked in {candidates:?}); run `cargo build` \
         (and/or `cargo build --release`) first"
    );
}

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", &find_c_so()),
        rust: Impl::load("Rust", &find_rust_so()),
    })
}

// ---------------------------------------------------------------------------
// stdout capture
//
// Both shared objects log through the *process's* libc `printf`, so stdout is
// captured by temporarily redirecting file descriptor 1 to a temp file and
// `fflush(NULL)`-ing afterwards. fd juggling is process-global, hence the
// mutex: it serialises every capturing call in the test binary.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Guards fd 1. EVERY call into either shared object goes through this lock,
/// because fd redirection is process-global: an unguarded `printf` from another
/// test thread would otherwise land inside someone else's capture.
static FD_LOCK: Mutex<()> = Mutex::new(());

/// Runs `f` with fd 1 redirected into `sink` and restores fd 1 afterwards.
/// Caller must already hold `FD_LOCK`.
unsafe fn with_fd1_redirected<R, F: FnOnce() -> R>(sink: c_int, f: F) -> R {
    unsafe {
        // Flush anything already pending so it does not land in the sink.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(sink, 1) >= 0, "dup2 failed");

        let result = f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        result
    }
}

/// Runs `f` with fd 1 redirected to a temp file and returns `(result, stdout)`.
pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    let _guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut tmp = tempfile();
    let result = unsafe { with_fd1_redirected(tmp.as_raw_fd(), f) };
    let mut buf = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("seek");
    tmp.read_to_end(&mut buf).expect("read");
    (result, buf)
}

/// Runs `f` with fd 1 sent to `/dev/null`. Used for the high-volume randomized
/// sweeps where only the return value is compared: it keeps the library's
/// `printf` output from polluting the test log AND from leaking into a
/// concurrent `capture_stdout`.
pub fn silence_stdout<R, F: FnOnce() -> R>(f: F) -> R {
    static DEVNULL: OnceLock<std::fs::File> = OnceLock::new();
    let devnull = DEVNULL.get_or_init(|| {
        std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .expect("open /dev/null")
    });
    let _guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { with_fd1_redirected(devnull.as_raw_fd(), f) }
}

fn tempfile() -> std::fs::File {
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = dir.join(format!(
        "gotomach-cap-{}-{}-{}.txt",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let f = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .unwrap_or_else(|e| panic!("cannot create temp file {}: {e}", path.display()));
    // Unlink immediately; the fd keeps it alive.
    let _ = std::fs::remove_file(&path);
    f
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

/// Calls `gotomach` in BOTH shared objects with identical arguments and asserts
/// the return value AND the printed log bytes match exactly.
pub fn diff_gotomach(iterations: c_int, seed: c_int, mode: c_int, threshold: c_int) -> c_int {
    let p = pair();
    let g_c = p.c.gotomach;
    let g_r = p.rust.gotomach;
    let (rc_c, out_c) = capture_stdout(|| unsafe { g_c(iterations, seed, mode, threshold) });
    let (rc_r, out_r) = capture_stdout(|| unsafe { g_r(iterations, seed, mode, threshold) });

    let ctx = format!("gotomach(iterations={iterations}, seed={seed}, mode={mode}, threshold={threshold})");
    assert_eq!(
        rc_c, rc_r,
        "return value mismatch for {ctx}\n  C stdout: {:?}\n  Rust stdout: {:?}",
        String::from_utf8_lossy(&out_c),
        String::from_utf8_lossy(&out_r),
    );
    assert_eq!(
        out_c,
        out_r,
        "stdout mismatch for {ctx}\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&out_c),
        String::from_utf8_lossy(&out_r),
    );
    rc_c
}

/// Like `diff_gotomach` but skips the (slow) stdout capture — for the bulk
/// randomized sweeps where the log content is already pinned by the dedicated
/// stdout rows.
pub fn diff_gotomach_rc(iterations: c_int, seed: c_int, mode: c_int, threshold: c_int) -> c_int {
    let p = pair();
    let (rc_c, rc_r) = silence_stdout(|| unsafe {
        (
            (p.c.gotomach)(iterations, seed, mode, threshold),
            (p.rust.gotomach)(iterations, seed, mode, threshold),
        )
    });
    assert_eq!(
        rc_c, rc_r,
        "return value mismatch for gotomach(iterations={iterations}, seed={seed}, \
         mode={mode}, threshold={threshold})"
    );
    rc_c
}

/// Differential call of one of the three `operation_fn` helpers.
pub fn diff_op(name: &str, value: c_int, unused_param: c_int, ctxp: *mut c_void) -> c_int {
    let p = pair();
    let a = unsafe { (p.c.op_by_name(name))(value, unused_param, ctxp) };
    let b = unsafe { (p.rust.op_by_name(name))(value, unused_param, ctxp) };
    assert_eq!(
        a, b,
        "{name}(value={value}, unused_param={unused_param}, ctx={ctxp:?}) mismatch: C={a} Rust={b}"
    );
    a
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seeds keep every run reproducible.
// ---------------------------------------------------------------------------
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

/// The `mode` values the switch distinguishes: the three valid arms plus a
/// spread of out-of-range "enum" values that all land in `default`.
pub const ALL_MODES: &[c_int] = &[0, 1, 2, -1, 3, 4, 99, i32::MIN, i32::MAX];
pub const VALID_MODES: &[c_int] = &[0, 1, 2];
pub const INVALID_MODES: &[c_int] = &[-1, 3, 4, 99, -12345, i32::MIN, i32::MAX];
