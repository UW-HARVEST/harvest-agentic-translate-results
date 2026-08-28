// Shared differential-test harness.
//
// BOTH implementations are loaded as shared objects with `libloading` and called
// exclusively through their exported C symbols -- the Rust crate is *never*
// linked directly, so the `#[no_mangle] extern "C"` wrappers are part of what is
// under test.
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture and crash-isolated calls.
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
}

// ---------------------------------------------------------------------------
// Signatures of the seven exported symbols.
// ---------------------------------------------------------------------------

pub type FnClassifyMode = unsafe extern "C" fn(*const c_char) -> c_int;
pub type FnApplyMultiplier = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnConvertDouble = unsafe extern "C" fn(f64) -> c_int;
pub type FnGetModifiedTime = unsafe extern "C" fn(c_int, c_int) -> i64;
pub type FnHashTimeValue = unsafe extern "C" fn(i64) -> c_int;
pub type FnModeselect = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _handle: libloading::Library,
    pub classify_mode: FnClassifyMode,
    pub apply_multiplier: FnApplyMultiplier,
    pub convert_time_factor: FnConvertDouble,
    pub convert_negative_overflow: FnConvertDouble,
    pub get_modified_time: FnGetModifiedTime,
    pub hash_time_value: FnHashTimeValue,
    pub modeselect: FnModeselect,
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        unsafe {
            let handle = libloading::Library::new(&path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
            macro_rules! sym {
                ($t:ty, $s:literal) => {{
                    let s: libloading::Symbol<$t> = handle
                        .get($s)
                        .unwrap_or_else(|e| panic!("{}: missing symbol {:?}: {e}", name, $s));
                    *s
                }};
            }
            let classify_mode = sym!(FnClassifyMode, b"classify_mode\0");
            let apply_multiplier = sym!(FnApplyMultiplier, b"apply_multiplier\0");
            let convert_time_factor = sym!(FnConvertDouble, b"convert_time_factor\0");
            let convert_negative_overflow = sym!(FnConvertDouble, b"convert_negative_overflow\0");
            let get_modified_time = sym!(FnGetModifiedTime, b"get_modified_time\0");
            let hash_time_value = sym!(FnHashTimeValue, b"hash_time_value\0");
            let modeselect = sym!(FnModeselect, b"modeselect\0");
            Lib {
                name,
                path,
                _handle: handle,
                classify_mode,
                apply_multiplier,
                convert_time_factor,
                convert_negative_overflow,
                get_modified_time,
                hash_time_value,
                modeselect,
            }
        }
    }
}

pub struct Libs {
    pub c: Lib,
    pub rs: Lib,
}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `<workspace>/c_src/build/lib<something>.so` -- the project name (and hence the
/// library file name) is derived by CMake from the parent directory name, so it
/// must be discovered, not hard-coded.
pub fn c_so_path() -> PathBuf {
    let build = manifest_dir().join("../c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                found.push(p);
            }
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {} (found {:?}); build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// `target/<profile>/libmodeselect_lib.so`, discovered relative to the running
/// test executable so that it always matches the profile under test.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test exe layout")
        .to_path_buf();
    let direct = profile_dir.join("libmodeselect_lib.so");
    if direct.exists() {
        return direct;
    }
    // Fall back to whichever profile directory happens to contain it.
    for prof in ["debug", "release"] {
        let p = manifest_dir().join("target").join(prof).join("libmodeselect_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libmodeselect_lib.so not found (looked in {}); run `cargo build` first",
        profile_dir.display()
    );
}

/// `cargo test` builds the *test* binaries but does **not** relink the crate's
/// `cdylib`, so it is entirely possible to run a full green test suite against a
/// stale `.so`.  (Observed in practice: a one-constant mutation survived in
/// `target/debug/libmodeselect_lib.so` while `src/lib.rs` was already restored.)
/// Refuse to run rather than report a meaningless result.
fn assert_not_stale(so: &std::path::Path) {
    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    for src in ["src/lib.rs", "Cargo.toml"] {
        let p = manifest_dir().join(src);
        if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            assert!(
                t <= so_mtime,
                "STALE ARTIFACT: {} is newer than {}.\n\
                 `cargo test` does not relink the cdylib -- run `cargo build` (and \
                 `cargo build --release` for the release profile) before testing.",
                p.display(),
                so.display()
            );
        }
    }
}

/// `target/<profile>/examples/libfaketime.so` -- the LD_PRELOAD `time()`
/// interposer, discovered relative to the running test executable.
pub fn faketime_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("test exe layout")
        .to_path_buf();
    let direct = profile_dir.join("examples").join("libfaketime.so");
    if direct.exists() {
        return direct;
    }
    for prof in ["debug", "release"] {
        let p = manifest_dir()
            .join("target")
            .join(prof)
            .join("examples")
            .join("libfaketime.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libfaketime.so not found (looked in {}); run `cargo build --examples` first",
        profile_dir.display()
    );
}

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let rs = rust_so_path();
        assert_not_stale(&rs);
        Libs {
            c: Lib::open("C", c_so_path()),
            rs: Lib::open("Rust", rs),
        }
    })
}

// ---------------------------------------------------------------------------
// stdout capture (in-process; fd 1 is restored afterwards).
// ---------------------------------------------------------------------------

static FD_LOCK: Mutex<()> = Mutex::new(());
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn scratch_dir() -> PathBuf {
    let d = manifest_dir().join("target").join("difftest");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn scratch_file(tag: &str) -> (PathBuf, File) {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let p = scratch_dir().join(format!("{}-{}-{}.bin", tag, std::process::id(), n));
    let f = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&p)
        .unwrap_or_else(|e| panic!("create {}: {e}", p.display()));
    (p, f)
}

/// Runs `f` with fd 1 redirected to a temporary file and returns
/// `(return value, bytes written to stdout)`.
///
/// The C and the Rust library both write through libc `printf`, so this compares
/// the *exact* bytes the two libraries emit, including `%.2e` rounding and `%X`
/// hex casing.
pub fn capture<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    let _guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (path, file) = scratch_file("stdout");
    let ret;
    let mut buf = Vec::new();
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
        ret = f();
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }
    let mut file = File::open(&path).expect("reopen capture file");
    file.read_to_end(&mut buf).expect("read capture file");
    drop(file);
    let _ = std::fs::remove_file(&path);
    (ret, buf)
}

// ---------------------------------------------------------------------------
// crash-isolated call (needed for the C code's genuine SIGSEGV paths).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Exit code, or `-signal` if the child was killed by a signal.
    pub status: i32,
    /// Value returned by the callee, valid only when `status == 0`.
    pub ret: i64,
    pub stdout: Vec<u8>,
}

impl Outcome {
    pub fn crashed_with(&self, sig: i32) -> bool {
        self.status == -sig
    }
    pub fn ok(&self) -> bool {
        self.status == 0
    }
}

fn decode_status(status: c_int) -> i32 {
    let sig = status & 0x7f;
    if sig == 0 {
        (status >> 8) & 0xff
    } else if sig == 0x7f {
        // stopped -- should not happen here
        -0x7f
    } else {
        -sig
    }
}

/// Calls `f` in a forked child with fd 1 redirected to a file, and reports the
/// child's exit status / termination signal, the returned value and the bytes it
/// printed.  Used for the inputs on which the C library legitimately faults.
pub fn run_isolated<F: FnOnce() -> i64>(f: F) -> Outcome {
    let _guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (out_path, out_file) = scratch_file("iso-out");
    let (ret_path, ret_file) = scratch_file("iso-ret");
    let out_fd = out_file.as_raw_fd();
    let ret_fd = ret_file.as_raw_fd();

    let status;
    unsafe {
        // Flush everything before forking so the child cannot duplicate our
        // buffered output.
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        fflush(std::ptr::null_mut());

        let pid = fork();
        assert!(pid >= 0, "fork() failed");
        if pid == 0 {
            // ---- child ----
            dup2(out_fd, 1);
            let r = f();
            fflush(std::ptr::null_mut());
            let bytes = r.to_ne_bytes();
            write(ret_fd, bytes.as_ptr() as *const c_void, 8);
            _exit(0);
        }
        let mut st: c_int = 0;
        let w = waitpid(pid, &mut st as *mut c_int, 0);
        assert_eq!(w, pid, "waitpid failed");
        status = decode_status(st);
    }
    drop(out_file);
    drop(ret_file);

    let stdout = std::fs::read(&out_path).unwrap_or_default();
    let retbuf = std::fs::read(&ret_path).unwrap_or_default();
    let ret = if retbuf.len() == 8 {
        i64::from_ne_bytes(retbuf.try_into().unwrap())
    } else {
        0
    };
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&ret_path);

    Outcome { status, ret, stdout }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) -- fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn next_i64(&mut self) -> i64 {
        self.next_u64() as i64
    }
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Uniform in `[lo, hi]` (inclusive), for `i32`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    /// Uniform in `[-1, 1]` times `scale`, always finite.
    pub fn unit_f64(&mut self) -> f64 {
        let m = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
        2.0 * m - 1.0
    }
    /// A finite `f64` drawn from random bit patterns (rejects NaN/inf).
    pub fn finite_f64(&mut self) -> f64 {
        loop {
            let v = f64::from_bits(self.next_u64());
            if v.is_finite() {
                return v;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Assertion helpers.
// ---------------------------------------------------------------------------

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

#[track_caller]
pub fn eq_int(row: &str, ctx: impl std::fmt::Debug, c: c_int, rs: c_int) {
    assert_eq!(
        c, rs,
        "[{row}] divergence for input {ctx:?}: C returned {c} (0x{c:X}), Rust returned {rs} (0x{rs:X})"
    );
}

#[track_caller]
pub fn eq_i64(row: &str, ctx: impl std::fmt::Debug, c: i64, rs: i64) {
    assert_eq!(
        c, rs,
        "[{row}] divergence for input {ctx:?}: C returned {c} (0x{c:X}), Rust returned {rs} (0x{rs:X})"
    );
}

#[track_caller]
pub fn eq_bytes(row: &str, ctx: impl std::fmt::Debug, c: &[u8], rs: &[u8]) {
    if c != rs {
        panic!(
            "[{row}] stdout divergence for input {ctx:?}\n  C   : {}\n  Rust: {}",
            show(c),
            show(rs)
        );
    }
}
