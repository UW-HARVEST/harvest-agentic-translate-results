//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls every function only
//! through its exported C symbol, so the `#[no_mangle]` export wrappers are part
//! of what is under test. No Rust function is ever called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type FnClassifyMode = unsafe extern "C" fn(*const c_char) -> i32;
pub type FnApplyMultiplier = unsafe extern "C" fn(i32, i32) -> i32;
pub type FnConvertF64 = unsafe extern "C" fn(f64) -> i32;
pub type FnGetModifiedTime = unsafe extern "C" fn(i32, i32) -> i64;
pub type FnHashTimeValue = unsafe extern "C" fn(i64) -> i32;
pub type FnModeselect = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;

/// One loaded implementation (either the C `.so` or the Rust `.so`).
pub struct Impl {
    pub name: &'static str,
    lib: Library,
}

impl Impl {
    fn open(name: &'static str, path: PathBuf) -> Impl {
        assert!(path.is_file(), "{} shared object not found: {:?}", name, path);
        // SAFETY: loading a shared object we just built ourselves.
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen {:?} failed: {e}", path));
        Impl { name, lib }
    }

    fn sym<T>(&self, s: &[u8]) -> Symbol<'_, T> {
        // SAFETY: signatures are taken from c_src/src/lib.c.
        unsafe { self.lib.get(s) }
            .unwrap_or_else(|e| panic!("{}: dlsym {:?} failed: {e}", self.name, String::from_utf8_lossy(s)))
    }

    pub fn classify_mode(&self) -> Symbol<'_, FnClassifyMode> {
        self.sym(b"classify_mode\0")
    }
    pub fn apply_multiplier(&self) -> Symbol<'_, FnApplyMultiplier> {
        self.sym(b"apply_multiplier\0")
    }
    pub fn convert_time_factor(&self) -> Symbol<'_, FnConvertF64> {
        self.sym(b"convert_time_factor\0")
    }
    pub fn convert_negative_overflow(&self) -> Symbol<'_, FnConvertF64> {
        self.sym(b"convert_negative_overflow\0")
    }
    pub fn get_modified_time(&self) -> Symbol<'_, FnGetModifiedTime> {
        self.sym(b"get_modified_time\0")
    }
    pub fn hash_time_value(&self) -> Symbol<'_, FnHashTimeValue> {
        self.sym(b"hash_time_value\0")
    }
    pub fn modeselect(&self) -> Symbol<'_, FnModeselect> {
        self.sym(b"modeselect\0")
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {:?} ({e}).\nBuild the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one .so in {:?}, found {:?}",
        build,
        candidates
    );
    candidates.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    // The test binary lives in target/<profile>/deps/; the cdylib is one level up.
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    for dir in [profile, deps] {
        let p = dir.join("libmodeselect_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "libmodeselect_lib.so not found near {:?}. Run `cargo build` first.",
        profile
    );
}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// The two loaded implementations. Both `.so`s stay loaded for the whole test
/// binary; `dlopen` with distinct paths gives distinct handles, and `dlsym` on a
/// handle resolves that object's own symbols even though the names collide.
pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::open("C", find_c_so()),
        rs: Impl::open("Rust", find_rust_so()),
    })
}

// ---------------------------------------------------------------------------
// deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    /// Fixed default seed so every run is reproducible.
    pub fn fixed() -> Rng {
        Rng(SEED)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
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
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Random `f64` from raw bits — covers NaN, infinities and subnormals.
    pub fn any_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// Log-uniform magnitude in `[10^lo, 10^hi]` with a random sign.
    pub fn log_f64(&mut self, lo: f64, hi: f64) -> f64 {
        let e = lo + self.unit() * (hi - lo);
        let m = 10f64.powf(e);
        if self.next_u64() & 1 == 0 { m } else { -m }
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Serializes stdout capture: fd 1 is process-global, so two threads
/// redirecting it concurrently would steal each other's output.
static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// How a forked child terminated.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signalled(i32),
}

/// Result of running one FFI call in an isolated child process.
pub struct ChildCall {
    pub outcome: Outcome,
    /// The value the callee returned, or `None` if the child died first.
    pub ret: Option<i64>,
    /// Exact bytes the callee wrote to fd 1.
    pub stdout: Vec<u8>,
}

/// glibc allocates `stdout`'s buffer lazily. Do it once in the parent, before
/// any fork, so a forked child never has to call `malloc` from inside `printf`
/// while another thread might hold the allocator lock.
fn prewarm_stdout() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        use std::os::unix::io::AsRawFd;
        let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: plain fd juggling, restored immediately.
        unsafe {
            libc::fflush(std::ptr::null_mut());
            let sink = std::fs::File::create("/dev/null").expect("open /dev/null");
            let saved = libc::dup(1);
            libc::dup2(sink.as_raw_fd(), 1);
            libc::printf(c"prewarm\n".as_ptr());
            libc::fflush(std::ptr::null_mut());
            libc::dup2(saved, 1);
            libc::close(saved);
        }
    });
}

/// Runs `f` in a forked child with fd 1 pointing at a fresh temp file, and
/// returns how the child terminated, what `f` returned, and everything `f`
/// printed.
///
/// Forking (rather than redirecting fd 1 in-process) is what makes the capture
/// exact: libtest's own progress lines are written to the real fd 1 by the
/// parent's main thread at unpredictable moments, and would otherwise land in
/// the middle of the captured bytes. It also lets the two rows where the C's
/// undefined behaviour crashes be compared by exit status.
pub fn call_in_child<F: FnOnce() -> i64>(f: F) -> ChildCall {
    use std::os::unix::io::AsRawFd;

    prewarm_stdout();

    let out_file = tempfile();
    let ret_file = tempfile();
    let out_fd = out_file.as_raw_fd();
    let ret_fd = ret_file.as_raw_fd();

    // SAFETY: the child only performs fd setup, the FFI call, one `write`, and
    // `_exit`; it never returns into the test harness.
    let outcome = unsafe {
        libc::fflush(std::ptr::null_mut());
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            libc::dup2(out_fd, 1);
            let v = f();
            libc::fflush(std::ptr::null_mut());
            let bytes = v.to_ne_bytes();
            libc::write(ret_fd, bytes.as_ptr() as *const libc::c_void, 8);
            libc::_exit(0);
        }
        let mut status: i32 = 0;
        assert!(libc::waitpid(pid, &mut status, 0) == pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            Outcome::Signalled(libc::WTERMSIG(status))
        } else {
            Outcome::Exited(libc::WEXITSTATUS(status))
        }
    };

    let stdout = read_all(out_file);
    let ret_bytes = read_all(ret_file);
    let ret = if ret_bytes.len() == 8 {
        Some(i64::from_ne_bytes(ret_bytes.try_into().unwrap()))
    } else {
        None
    };

    ChildCall { outcome, ret, stdout }
}

fn read_all(f: std::fs::File) -> Vec<u8> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = f;
    f.seek(SeekFrom::Start(0)).expect("seek");
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).expect("read temp file");
    buf
}

/// Runs `f` with fd 1 redirected to a temporary file and returns `f`'s result
/// together with the exact bytes written to stdout.
///
/// Both `.so`s write through the *same* process-wide libc `stdout` FILE*, so
/// `fflush(NULL)` before restoring fd 1 is what makes the capture exact.
///
/// NOTE: prefer [`call_in_child`] when the captured bytes are compared, because
/// libtest may write its own progress output to fd 1 concurrently.
pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    use std::os::unix::io::AsRawFd;

    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // SAFETY: plain fd juggling; every raw fd is closed or restored below.
    unsafe {
        libc::fflush(std::ptr::null_mut());

        let tmp = tempfile();
        let tmp_fd = tmp.as_raw_fd();

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(tmp_fd, 1) >= 0, "dup2 onto stdout failed");

        let out = f();

        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "restoring stdout failed");
        libc::close(saved);

        let mut buf = Vec::new();
        {
            use std::io::{Read, Seek, SeekFrom};
            let mut tmp = tmp;
            tmp.seek(SeekFrom::Start(0)).expect("seek");
            tmp.read_to_end(&mut buf).expect("read captured stdout");
        }
        (out, buf)
    }
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "modeselect-diff-{}-{}.out",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create temp file for stdout capture");
    // Unlink immediately; the open handle keeps it alive.
    let _ = std::fs::remove_file(&path);
    f
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// Builds a NUL-terminated byte vector usable as a `const char *`.
pub fn cstr(bytes: &[u8]) -> Vec<c_char> {
    assert!(!bytes.contains(&0), "input must not contain interior NUL");
    let mut v: Vec<c_char> = bytes.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// modeselect differential helper (fork-isolated, stdout + return value)
// ---------------------------------------------------------------------------

/// Calls `modeselect` in the C `.so` and in the Rust `.so`, each in its own
/// child process, and returns both results.
pub fn modeselect_both(ms: i32, to: i32, cx: i32, seed: i32) -> (ChildCall, ChildCall) {
    let p = pair();
    let c = p.c.modeselect();
    let rs = p.rs.modeselect();
    // SAFETY: plain scalar FFI calls, performed in child processes.
    let a = call_in_child(|| unsafe { c(ms, to, cx, seed) } as i64);
    let b = call_in_child(|| unsafe { rs(ms, to, cx, seed) } as i64);
    (a, b)
}

/// Asserts the C and Rust `modeselect` agree on return value AND on every byte
/// printed to stdout.
///
/// `mode_selector` must satisfy `mode_selector % 4 >= 0`; see ERRORS.md row 32.
pub fn diff_modeselect(tag: &str, ms: i32, to: i32, cx: i32, seed: i32) {
    assert!(
        (0..4).contains(&(ms % 4)),
        "test bug: mode_selector={ms} (%4={}) indexes out of bounds in the C",
        ms % 4
    );
    let args = format!("mode_selector={ms} time_offset={to} complexity={cx} seed={seed}");
    let (mut a, mut b) = modeselect_both(ms, to, cx, seed);

    if a.stdout != b.stdout || a.ret != b.ret {
        // `Modified time` derives from `time(NULL) >> 29`, which ticks once every
        // ~17 years. If a tick landed between the two calls, re-run once.
        let (a2, b2) = modeselect_both(ms, to, cx, seed);
        a = a2;
        b = b2;
    }

    assert_eq!(
        a.outcome, b.outcome,
        "modeselect TERMINATION divergence [{tag}] {args}"
    );
    assert_eq!(
        show(&a.stdout),
        show(&b.stdout),
        "modeselect STDOUT divergence [{tag}] {args}"
    );
    assert_eq!(a.ret, b.ret, "modeselect RETURN divergence [{tag}] {args}");
    assert_eq!(
        a.outcome,
        Outcome::Exited(0),
        "modeselect should return normally [{tag}] {args}"
    );
    // Guard against a regression where both print nothing and thus "match".
    assert_eq!(
        a.stdout.iter().filter(|&&x| x == b'\n').count(),
        9,
        "unexpected captured line count [{tag}] {args}: {}",
        show(&a.stdout)
    );
}
