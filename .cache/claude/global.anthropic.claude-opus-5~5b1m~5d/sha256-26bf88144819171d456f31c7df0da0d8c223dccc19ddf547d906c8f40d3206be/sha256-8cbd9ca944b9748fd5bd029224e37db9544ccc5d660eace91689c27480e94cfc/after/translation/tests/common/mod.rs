// Shared differential-test harness.
//
// Loads BOTH shared libraries through `libloading` and calls the exported
// `sieve` symbol on each, exactly as an external C consumer would. The Rust
// implementation is NEVER called directly, so the `#[no_mangle]`/`extern "C"`
// export wrapper is part of what is under test.
//
// `sieve` returns `void` and its only observable effect is what it writes to
// stdout via `printf`, so "compare the outputs" means "capture fd 1 around each
// call and compare the bytes".

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub type SieveFn = unsafe extern "C" fn(libc::c_int);

/// Which of the two libraries to invoke.
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

pub struct Libs {
    pub c: Library,
    pub rust: Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Libs {
    pub fn sieve(&self, which: Impl) -> Symbol<'_, SieveFn> {
        let lib = match which {
            Impl::C => &self.c,
            Impl::Rust => &self.rust,
        };
        unsafe {
            lib.get(b"sieve\0")
                .unwrap_or_else(|e| panic!("{} .so does not export `sieve`: {e}", which.name()))
        }
    }

    /// Call `sieve(val)` through the given library's dynamic symbol.
    pub fn call(&self, which: Impl, val: i32) {
        let f = self.sieve(which);
        unsafe { f(val as libc::c_int) }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_c_so() -> PathBuf {
    if let Ok(p) = std::env::var("C_SIEVE_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir();
    let candidates = [
        root.join("../c_src/build/libSieve.so"),
        root.join("../c_src/build/lib/libSieve.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "could not find the C shared library. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         (or set $C_SIEVE_SO). Looked in: {candidates:?}"
    );
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SIEVE_SO") {
        return PathBuf::from(p);
    }
    // The test executable lives at <target>/<profile>/deps/<name>-<hash>.
    let exe = std::env::current_exe().expect("current_exe");
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(profile_dir) = exe.parent().and_then(Path::parent) {
        dirs.push(profile_dir.to_path_buf());
        if let Some(target_root) = profile_dir.parent() {
            dirs.push(target_root.join("debug"));
            dirs.push(target_root.join("release"));
        }
    }
    dirs.push(manifest_dir().join("target/debug"));
    dirs.push(manifest_dir().join("target/release"));
    for d in &dirs {
        let c = d.join("libSieve.so");
        if c.exists() {
            return c;
        }
    }
    panic!(
        "could not find the Rust cdylib libSieve.so. Build it with `cargo build` \
         (or set $RUST_SIEVE_SO). Looked in: {dirs:?}"
    );
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", rust_path.display()));
        // Sanity: both must export `sieve`.
        unsafe {
            let _: Symbol<SieveFn> = c.get(b"sieve\0").expect("C .so exports sieve");
            let _: Symbol<SieveFn> = rust.get(b"sieve\0").expect("Rust .so exports sieve");
        }
        Libs {
            c,
            rust,
            c_path,
            rust_path,
        }
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

fn tmp_path(tag: &str) -> PathBuf {
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let n = std::process::id();
    let seq = {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        SEQ.fetch_add(1, Ordering::Relaxed)
    };
    PathBuf::from(dir).join(format!("sieve-diff-{n}-{tag}-{seq}.out"))
}

/// fd 1 is process-global, and `cargo test` runs test functions on several
/// threads, so every operation that redirects/forks stdout must be serialized.
fn io_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// watchdog
// ---------------------------------------------------------------------------
//
// A mistranslation can turn a terminating C loop into an infinite Rust loop
// (e.g. incrementing by 2, so no value ever ends in 9). Captured in-process,
// that hangs the whole test run and reports nothing, which would make the suite
// silently blind to that entire class of bug. The watchdog turns such a hang
// into a loud, attributed failure.

static WD_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static WD_SINCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static WD_DESC: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// Hard limit for a single `sieve` invocation captured in-process. Every input
/// class the suite drives in-process is bounded to < 1s of work, so anything
/// past this is a non-terminating divergence.
const WATCHDOG_SECS: u64 = 25;

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn watchdog_start() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        std::thread::spawn(|| {
            use std::sync::atomic::Ordering;
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let since = WD_SINCE.load(Ordering::Acquire);
                if since == 0 {
                    continue;
                }
                if now_secs().saturating_sub(since) > WATCHDOG_SECS {
                    let desc = WD_DESC.lock().map(|g| g.clone()).unwrap_or_default();
                    // stderr is fd 2 and untouched by the capture machinery.
                    eprintln!(
                        "\n*** WATCHDOG: `{desc}` has not returned after {WATCHDOG_SECS}s.\n\
                         *** The call did not terminate -- C and Rust DIVERGE (the C loop \
                         terminates for this input).\n*** Aborting the test process."
                    );
                    std::process::abort();
                }
            }
        });
    });
}

fn watchdog_enter(desc: String) -> u64 {
    use std::sync::atomic::Ordering;
    watchdog_start();
    if let Ok(mut g) = WD_DESC.lock() {
        *g = desc;
    }
    WD_SINCE.store(now_secs().max(1), Ordering::Release);
    WD_SEQ.fetch_add(1, Ordering::AcqRel)
}

fn watchdog_leave() {
    WD_SINCE.store(0, std::sync::atomic::Ordering::Release);
}

fn flush_everything() {
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    // fflush(NULL) flushes every open C output stream, including the `stdout`
    // FILE* that both libraries' printf calls write through.
    unsafe { libc::fflush(std::ptr::null_mut()) };
}

/// Run `f` with fd 1 redirected to a temp file; return everything it wrote.
///
/// Used for inputs whose output is bounded (the overwhelming majority).
pub fn capture<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let _guard = io_lock();
    let _wd = watchdog_enter(format!("capture({tag})"));
    let path = tmp_path(tag);
    let cpath = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    flush_everything();
    unsafe {
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = libc::open(
            cpath.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
            0o600,
        );
        assert!(fd >= 0, "open({}) failed", path.display());
        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");
        libc::close(fd);

        f();

        // Push printf's buffer out before we take fd 1 back.
        libc::fflush(std::ptr::null_mut());
        assert!(libc::dup2(saved, 1) >= 0, "dup2 restore failed");
        libc::close(saved);
    }
    watchdog_leave();
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Call `sieve(val)` on `which` and return the exact bytes it wrote to stdout.
pub fn run(which: Impl, val: i32) -> Vec<u8> {
    let l = libs();
    capture(&format!("{}-sieve({val})", which.name()), || {
        l.call(which, val)
    })
}

/// Call `sieve(v)` for every `v` in `vals`, in order, on the same library
/// handle, capturing the concatenated output of the whole sequence.
pub fn run_seq(which: Impl, vals: &[i32]) -> Vec<u8> {
    let l = libs();
    capture(&format!("{}-seq[{} vals]", which.name(), vals.len()), || {
        for &v in vals {
            l.call(which, v);
        }
    })
}

// ---------------------------------------------------------------------------
// bounded capture for inputs whose output is unbounded (overflow / INT_MIN)
// ---------------------------------------------------------------------------

pub struct Bounded {
    pub prefix: Vec<u8>,
    /// true if the child terminated on its own before we had to kill it
    pub finished: bool,
}

/// Run `sieve(val)` in a forked child with stdout redirected to a file, wait
/// until at least `limit` bytes have been produced (or the child exits, or a
/// timeout elapses), kill the child and return the first `limit` bytes.
///
/// Needed for `INT_MIN` and the signed-overflow window near `INT_MAX`, whose
/// C behaviour is an effectively unbounded stream of ~2^32 lines.
pub fn capture_prefix(which: Impl, val: i32, limit: usize) -> Bounded {
    let _guard = io_lock();
    let l = libs();
    let path = tmp_path(&format!("{}-prefix", which.name()));
    let cpath = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    flush_everything();

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // ---- child ----
        unsafe {
            let fd = libc::open(
                cpath.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
                0o600,
            );
            if fd < 0 {
                libc::_exit(101);
            }
            if libc::dup2(fd, 1) < 0 {
                libc::_exit(102);
            }
            libc::close(fd);
            l.call(which, val);
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }
    }

    // ---- parent ----
    let want = (limit as u64).saturating_add(1 << 16);
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut finished = false;
    loop {
        let sz = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if sz >= want {
            break;
        }
        let mut status: libc::c_int = 0;
        let r = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if r == pid {
            finished = true;
            break;
        }
        if Instant::now() > deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    if !finished {
        unsafe {
            libc::kill(pid, libc::SIGKILL);
            let mut status: libc::c_int = 0;
            libc::waitpid(pid, &mut status, 0);
        }
    }

    let mut bytes = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    if bytes.len() > limit {
        bytes.truncate(limit);
    }
    Bounded {
        prefix: bytes,
        finished,
    }
}

/// Outcome of running `sieve` in a child process with stdout closed.
#[derive(Debug, PartialEq, Eq)]
pub struct ChildOutcome {
    /// `Some(code)` for a normal exit, `None` if killed by a signal
    pub exit_code: Option<i32>,
    /// `Some(sig)` if killed by a signal
    pub signal: Option<i32>,
    /// true if we had to SIGKILL it because it did not terminate in time
    pub timed_out: bool,
}

/// Run `sieve(val)` in a forked child whose fd 1 has been **closed**, so every
/// `printf` fails. The C code ignores `printf`'s return value, so the call must
/// still terminate normally. Returns how the child died.
pub fn run_with_closed_stdout(which: Impl, val: i32) -> ChildOutcome {
    let _guard = io_lock();
    let l = libs();
    flush_everything();
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        unsafe {
            libc::close(1);
            l.call(which, val);
            // Reaching here means the loop terminated, which is the point.
            libc::_exit(7);
        }
    }

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let mut status: libc::c_int = 0;
        let r = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if r == pid {
            let exited = libc::WIFEXITED(status);
            return ChildOutcome {
                exit_code: if exited {
                    Some(libc::WEXITSTATUS(status))
                } else {
                    None
                },
                signal: if libc::WIFSIGNALED(status) {
                    Some(libc::WTERMSIG(status))
                } else {
                    None
                },
                timed_out: false,
            };
        }
        if Instant::now() > deadline {
            unsafe {
                libc::kill(pid, libc::SIGKILL);
                let mut s: libc::c_int = 0;
                libc::waitpid(pid, &mut s, 0);
            }
            return ChildOutcome {
                exit_code: None,
                signal: None,
                timed_out: true,
            };
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Run `sieve(val)` in a forked child whose stdout is a **pipe** (different
/// buffering regime than a regular file), returning what came out of the pipe.
pub fn run_through_pipe(which: Impl, val: i32) -> Vec<u8> {
    let _guard = io_lock();
    let l = libs();
    flush_everything();
    let mut fds = [0 as libc::c_int; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe failed");
    let (rd, wr) = (fds[0], fds[1]);

    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        unsafe {
            libc::close(rd);
            if libc::dup2(wr, 1) < 0 {
                libc::_exit(102);
            }
            libc::close(wr);
            l.call(which, val);
            libc::fflush(std::ptr::null_mut());
            libc::_exit(0);
        }
    }
    unsafe { libc::close(wr) };

    // Cap the read so a non-terminating (mistranslated) child cannot hang the
    // test run: exceeding the cap is itself a detectable divergence, because the
    // C side terminates well inside it for every input this helper is used with.
    const CAP: usize = 8 * 1024 * 1024;
    let mut out = Vec::new();
    let mut buf = [0u8; 8192];
    while out.len() < CAP {
        let n = unsafe { libc::read(rd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&buf[..n as usize]);
    }
    unsafe {
        libc::kill(pid, libc::SIGKILL);
        libc::close(rd);
        let mut s: libc::c_int = 0;
        libc::waitpid(pid, &mut s, 0);
    }
    out
}

// ---------------------------------------------------------------------------
// comparison
// ---------------------------------------------------------------------------

fn snippet(b: &[u8], at: usize) -> String {
    let lo = at.saturating_sub(40);
    let hi = (at + 40).min(b.len());
    String::from_utf8_lossy(&b[lo..hi]).escape_debug().to_string()
}

/// Byte-for-byte comparison with a compact, actionable failure message.
pub fn assert_same_bytes(ctx: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    let at = c
        .iter()
        .zip(r.iter())
        .position(|(a, b)| a != b)
        .unwrap_or(c.len().min(r.len()));
    panic!(
        "{ctx}: C and Rust stdout differ\n  \
         C len   = {}\n  Rust len = {}\n  \
         first difference at byte {at}\n  \
         C   ...{}...\n  Rust ...{}...",
        c.len(),
        r.len(),
        snippet(c, at),
        snippet(r, at),
    );
}

/// Compare C vs Rust for one input value (bounded output).
pub fn assert_match(val: i32) {
    let c = run(Impl::C, val);
    let r = run(Impl::Rust, val);
    assert_same_bytes(&format!("sieve({val})"), &c, &r);
}

/// Compare C vs Rust across a whole set of input values.
pub fn assert_match_all(label: &str, vals: &[i32]) {
    for &v in vals {
        let c = run(Impl::C, v);
        let r = run(Impl::Rust, v);
        assert_same_bytes(&format!("[{label}] sieve({v})"), &c, &r);
    }
}

// ---------------------------------------------------------------------------
// deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_C0DE_5127_0001;

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
}

/// Reference model of the C loop, used only to *bound* how much output a
/// randomly chosen input will produce (never to decide correctness).
pub fn line_count(val: i32) -> Option<u64> {
    if val % 10 == 9 {
        return Some(1);
    }
    if val < 0 {
        // counts up to 9
        return Some((9i64 - val as i64 + 1) as u64);
    }
    // positive, not ending in 9: next value ending in 9 is reachable unless it
    // would overflow.
    let target = (val as i64) - (val as i64) % 10 + 9;
    if target > i32::MAX as i64 {
        None // overflow window: unbounded
    } else {
        Some((target - val as i64 + 1) as u64)
    }
}
