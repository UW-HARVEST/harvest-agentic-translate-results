// Shared differential-test harness.
//
// Both the C library and the Rust library are loaded as shared objects via
// `libloading`. The Rust side is NEVER called directly as a Rust function; every
// call goes through `dlsym` on `libmodeselect_lib.so`, exactly as an external C
// consumer would. That means the `#[no_mangle] extern "C"` export wrappers are
// under test too.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type TimeT = i64;

pub type FnClassifyMode = unsafe extern "C" fn(*const c_char) -> c_int;
pub type FnApplyMultiplier = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnConvertTimeFactor = unsafe extern "C" fn(c_double) -> c_int;
pub type FnConvertNegOverflow = unsafe extern "C" fn(c_double) -> c_int;
pub type FnGetModifiedTime = unsafe extern "C" fn(c_int, c_int) -> TimeT;
pub type FnHashTimeValue = unsafe extern "C" fn(TimeT) -> c_int;
pub type FnModeselect = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub classify_mode: FnClassifyMode,
    pub apply_multiplier: FnApplyMultiplier,
    pub convert_time_factor: FnConvertTimeFactor,
    pub convert_negative_overflow: FnConvertNegOverflow,
    pub get_modified_time: FnGetModifiedTime,
    pub hash_time_value: FnHashTimeValue,
    pub modeselect: FnModeselect,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        // SAFETY: loading a library we just built; its initializers are benign.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {} ({}) failed: {e}", path.display(), name));
        // SAFETY: each symbol exists in both libraries with the C ABI signature
        // declared above (verified by SYMBOLS.md / `nm -D`). The function
        // pointers are copied out of the `Symbol` guards, and `lib` is kept
        // alive in the same struct, so they stay valid for the struct's life.
        unsafe {
            macro_rules! sym {
                ($t:ty, $s:literal) => {{
                    let s: Symbol<$t> = lib
                        .get(concat!($s, "\0").as_bytes())
                        .unwrap_or_else(|e| panic!("dlsym {} in {} failed: {e}", $s, name));
                    *s
                }};
            }
            Impl {
                name,
                classify_mode: sym!(FnClassifyMode, "classify_mode"),
                apply_multiplier: sym!(FnApplyMultiplier, "apply_multiplier"),
                convert_time_factor: sym!(FnConvertTimeFactor, "convert_time_factor"),
                convert_negative_overflow: sym!(FnConvertNegOverflow, "convert_negative_overflow"),
                get_modified_time: sym!(FnGetModifiedTime, "get_modified_time"),
                hash_time_value: sym!(FnHashTimeValue, "hash_time_value"),
                modeselect: sym!(FnModeselect, "modeselect"),
                _lib: lib,
            }
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let n = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if n.starts_with("lib") && n.ends_with(".so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no lib*.so in {}; build the C first:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // Integration tests run from target/<profile>/deps/<bin>, so the cdylib for
    // the profile UNDER TEST sits two levels up. Derive the profile from the test
    // executable's own path rather than guessing, otherwise a `cargo test`
    // (debug) run would silently exercise a stale release `.so` and the
    // overflow-checked debug build would never be verified.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|deps| deps.parent()) {
            let p = profile_dir.join("libmodeselect_lib.so");
            if p.exists() {
                return p;
            }
        }
    }
    let base = workspace_root().join("translation/target");
    for prof in ["release", "debug"] {
        let p = base.join(prof).join("libmodeselect_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libmodeselect_lib.so not found under {}; run `cargo build --release` first",
        base.display()
    );
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// Path to the C `.so` under test.
pub fn c_so_path() -> PathBuf {
    find_c_so()
}

/// Path to the Rust `.so` under test, for the profile currently being tested.
pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", &find_c_so()),
        rs: Impl::load("Rust", &find_rust_so()),
    })
}

// ---------------------------------------------------------------------------
// deterministic PRNG (fixed seed, reproducible)
// ---------------------------------------------------------------------------

/// SplitMix64. Fixed seed so every run is reproducible.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

impl Rng {
    pub fn new() -> Rng {
        Rng(SEED)
    }
    pub fn with_seed(s: u64) -> Rng {
        Rng(s)
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
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    /// Uniform f64 in `[-1.0, 1.0)`.
    pub fn unit_f64(&mut self) -> f64 {
        let m = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        m * 2.0 - 1.0
    }
    /// Arbitrary bit pattern reinterpreted as f64 — yields NaNs, subnormals, inf.
    pub fn bits_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// A value spanning the whole exponent ladder, both signs.
    pub fn ladder_f64(&mut self) -> f64 {
        let exp = self.range_i32(-320, 308);
        let sign = if self.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        let mant = self.unit_f64().abs() * 9.0 + 1.0;
        sign * mant * 10f64.powi(exp)
    }
}

// ---------------------------------------------------------------------------
// stdout capture (both libraries share the process's libc stdout)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn write(fd: c_int, buf: *const std::ffi::c_void, n: usize) -> isize;
}

fn flush_stdout() {
    // SAFETY: NULL flushes all open output streams, which is what we want since
    // the C `.so` and the Rust `.so` write through the same libc stdio.
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

/// Run `f` in a FORKED CHILD with fd 1 redirected to a temp file, and return
/// everything it wrote.
///
/// Forking is what makes this correct under `cargo test`'s default parallel
/// harness: libtest writes its own progress lines ("test foo ... ok") straight
/// to fd 1 from other threads, so redirecting fd 1 in-process captures that
/// noise as well and produces spurious mismatches. A forked child gets its own
/// copy of the file-descriptor table, so the redirect is invisible to every
/// other test thread and the capture contains ONLY what `f` printed.
///
/// The child calls `f`, flushes, and `_exit`s, so no destructors, atexit
/// handlers, or libtest teardown can contribute output.
pub fn capture_forked<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    let mut tmp = tempfile();
    let tmpfd = tmp.as_raw_fd();

    // Flush the parent's buffers first so nothing pending is duplicated into
    // the child and written to the capture file.
    {
        use std::io::Write;
        std::io::stdout().flush().ok();
        std::io::stderr().flush().ok();
    }
    flush_stdout();

    // SAFETY: the child path performs only fd juggling, the caller's single FFI
    // call, `fflush`, and `_exit` -- it never allocates or takes a lock, so the
    // usual fork-in-a-threaded-process hazards do not apply. The parent only
    // waits.
    let status = unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            if dup2(tmpfd, 1) < 0 {
                _exit(101);
            }
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        let mut status: c_int = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        status
    };

    // WIFSIGNALED(status) => low 7 bits are the signal number.
    let sig = status & 0x7F;
    assert!(
        sig == 0 || sig == 0x7F,
        "capture child was killed by signal {sig}"
    );
    let code = (status >> 8) & 0xFF;
    assert_ne!(code, 101, "capture child failed to redirect stdout");

    let mut buf = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("seek temp");
    tmp.read_to_end(&mut buf).expect("read temp");
    buf
}

/// Like [`capture_forked`], but `f` also returns an `int` which is smuggled back
/// to the parent through a second, non-redirected temp file.
///
/// This gives a single forked call that yields BOTH the return value and the
/// exact stdout bytes, so the two are guaranteed to come from the same
/// invocation.
pub fn capture_forked_i32<F: FnOnce() -> c_int>(f: F) -> (c_int, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::io::AsRawFd;

    let mut tmp = tempfile();
    let mut val = tempfile();
    let tmpfd = tmp.as_raw_fd();
    let valfd = val.as_raw_fd();

    {
        use std::io::Write;
        std::io::stdout().flush().ok();
        std::io::stderr().flush().ok();
    }
    flush_stdout();

    // SAFETY: as `capture_forked`; the extra `write` of 4 bytes to a private fd
    // is async-signal-safe.
    let status = unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            if dup2(tmpfd, 1) < 0 {
                _exit(101);
            }
            let v = f();
            fflush(std::ptr::null_mut());
            let bytes = v.to_ne_bytes();
            if write(valfd, bytes.as_ptr() as *const std::ffi::c_void, 4) != 4 {
                _exit(102);
            }
            _exit(0);
        }
        let mut status: c_int = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        status
    };

    let sig = status & 0x7F;
    assert!(
        sig == 0 || sig == 0x7F,
        "capture child was killed by signal {sig}"
    );
    let code = (status >> 8) & 0xFF;
    assert_ne!(code, 101, "capture child failed to redirect stdout");
    assert_ne!(code, 102, "capture child failed to report its return value");

    let mut out = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("seek temp");
    tmp.read_to_end(&mut out).expect("read temp");

    let mut vb = Vec::new();
    val.seek(SeekFrom::Start(0)).expect("seek val");
    val.read_to_end(&mut vb).expect("read val");
    assert_eq!(vb.len(), 4, "child did not write a 4-byte return value");
    let v = c_int::from_ne_bytes([vb[0], vb[1], vb[2], vb[3]]);

    (v, out)
}

/// In-process fd-1 redirect. Only safe when the test harness is single
/// threaded; prefer [`capture_forked`] for output comparison. Kept for
/// swallowing output when the bytes are not being compared.
pub fn capture_stdout<T, F: FnOnce() -> T>(f: F) -> (T, Vec<u8>) {
    use std::io::{Read, Seek, SeekFrom};
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut tmp = tempfile();
    // SAFETY: plain fd juggling on fds we own; every branch restores fd 1.
    let (saved, tmpfd) = unsafe {
        use std::os::unix::io::AsRawFd;
        flush_stdout();
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let tmpfd = tmp.as_raw_fd();
        assert!(dup2(tmpfd, 1) >= 0, "dup2 onto stdout failed");
        (saved, tmpfd)
    };
    let _ = tmpfd;
    let out = f();
    // SAFETY: restoring the fd we saved above.
    unsafe {
        flush_stdout();
        assert!(dup2(saved, 1) >= 0, "dup2 restoring stdout failed");
        close(saved);
    }
    let mut buf = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("seek temp");
    tmp.read_to_end(&mut buf).expect("read temp");
    (out, buf)
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "modeselect_diff_{}_{}_{}.out",
        std::process::id(),
        n,
        SEED
    ));
    let f = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("create temp capture file");
    // Unlink immediately; the fd keeps it alive.
    let _ = std::fs::remove_file(&path);
    f
}

// ---------------------------------------------------------------------------
// assertion helpers
// ---------------------------------------------------------------------------

#[track_caller]
pub fn eq_int(row: &str, ctx: impl std::fmt::Display, c: c_int, r: c_int) {
    assert_eq!(
        c, r,
        "[{row}] divergence for {ctx}: C returned {c} (0x{c:08X}), Rust returned {r} (0x{r:08X})"
    );
}

#[track_caller]
pub fn eq_time(row: &str, ctx: impl std::fmt::Display, c: TimeT, r: TimeT) {
    assert_eq!(
        c, r,
        "[{row}] divergence for {ctx}: C returned {c} (0x{c:016X}), Rust returned {r} (0x{r:016X})"
    );
}

#[track_caller]
pub fn eq_bytes(row: &str, ctx: impl std::fmt::Display, c: &[u8], r: &[u8]) {
    if c != r {
        panic!(
            "[{row}] stdout divergence for {ctx}\n--- C   ({} bytes) ---\n{}\n--- Rust ({} bytes) ---\n{}\n",
            c.len(),
            String::from_utf8_lossy(c),
            r.len(),
            String::from_utf8_lossy(r)
        );
    }
}

/// Build a NUL-terminated buffer from raw bytes.
pub fn cstr(bytes: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = bytes.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}
