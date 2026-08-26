// Shared differential-test harness.
//
// Loads BOTH the C `.so` and the Rust `.so` through `libloading` and drives them
// only through their exported C symbols (`run`, `driver`) — never by calling
// Rust functions directly, so the `#[no_mangle]` export wrappers are under test
// too.
//
// Both public functions return `void`; their entire observable behaviour is the
// bytes they write to `stdout` (and, for a NULL input, the signal that kills the
// process). So the harness:
//
//   1. loads both libraries in the PARENT and never calls them there, leaving
//      each library's private `static the_house` pristine;
//   2. `fork()`s once per scenario, so every scenario starts from that pristine
//      state and scenarios cannot contaminate each other;
//   3. in the child, redirects fd 1 to a temp file, replays the whole op
//      sequence against ONE library, `fflush`es, restores fd 1, and repeats for
//      the other library (each library owns an independent `the_house`, so
//      running them back to back keeps their states in lockstep);
//   4. compares the two transcripts byte-for-byte in the parent.
//
// The child is deliberately allocation-free: temp files are opened and input
// strings are converted to `CString` in the parent before forking.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type RunFn = unsafe extern "C" fn(c_int);
pub type DriverFn = unsafe extern "C" fn(*const c_char);

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("c_src").join("build").join("libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    // current_exe is target/<profile>/deps/<testname>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    for cand in [
        deps.parent().map(|p| p.join("libdriver.so")),
        Some(deps.join("libdriver.so")),
    ]
    .into_iter()
    .flatten()
    {
        if cand.exists() {
            return cand;
        }
    }
    panic!(
        "Rust cdylib libdriver.so not found near {}.\n\
         The crate is cdylib-only, so integration tests do not link it and \
         `cargo test` alone will not build it.\n\
         Run `cargo build` (add --release for the release profile) first, or use \
         ./verify_all.sh which does this automatically.",
        deps.display()
    );
}

// ---------------------------------------------------------------------------
// Loaded implementation pair
// ---------------------------------------------------------------------------

pub struct Impl {
    pub name: &'static str,
    // Keep the Library alive for the whole process; the fn pointers below point into it.
    _lib: Library,
    pub run: RunFn,
    pub driver: DriverFn,
}

impl Impl {
    fn load(name: &'static str, path: &Path) -> Impl {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {}", path.display(), e));
            let run: Symbol<RunFn> = lib
                .get(b"run\0")
                .unwrap_or_else(|e| panic!("{}: missing symbol `run`: {}", name, e));
            let driver: Symbol<DriverFn> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{}: missing symbol `driver`: {}", name, e));
            let run = *run;
            let driver = *driver;
            Impl {
                name,
                _lib: lib,
                run,
                driver,
            }
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

unsafe impl Sync for Pair {}
unsafe impl Send for Pair {}

static PAIR: OnceLock<Pair> = OnceLock::new();

/// Load both libraries exactly once per test process. The libraries are never
/// invoked from the parent, so every forked scenario observes the pristine
/// `the_house = {floors: 2, bedrooms: 5, bathrooms: 2.5}`.
pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Impl::load("C", &c_so_path()),
        rust: Impl::load("Rust", &rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// Op sequences
// ---------------------------------------------------------------------------

/// One call into the library under test.
pub enum Op {
    /// `run(extra_bedrooms)` — the low-level entry point.
    Run(c_int),
    /// `driver(in)` — the high-level entry point. Holds an owned NUL-terminated
    /// string so the child never has to allocate.
    Driver(CString),
    /// `driver(NULL)` — only valid in crash tests.
    DriverNull,
    /// `driver(ptr)` where `ptr` points at these raw bytes verbatim. The buffer
    /// MUST already contain its own terminating NUL; this allows exercising
    /// interior-NUL buffers that `CString` forbids.
    DriverRaw(Vec<u8>),
}

impl Op {
    pub fn driver(s: &str) -> Op {
        Op::Driver(CString::new(s).expect("op string must not contain interior NUL"))
    }
    /// Build a `driver` op from raw bytes (allows exercising odd byte values).
    pub fn driver_bytes(b: &[u8]) -> Op {
        Op::Driver(CString::new(b).expect("op bytes must not contain interior NUL"))
    }
    /// Build a `driver` op over a buffer that may contain interior NULs. A
    /// terminating NUL is appended so the pointer is always valid to read.
    pub fn driver_raw(b: &[u8]) -> Op {
        let mut v = b.to_vec();
        v.push(0);
        Op::DriverRaw(v)
    }
    fn describe(&self) -> String {
        match self {
            Op::Run(v) => format!("run({})", v),
            Op::Driver(s) => format!("driver({:?})", s.to_string_lossy()),
            Op::DriverNull => "driver(NULL)".to_string(),
            Op::DriverRaw(v) => format!("driver(raw {:?})", String::from_utf8_lossy(v)),
        }
    }
}

/// Separator emitted to the captured stream before each op so the parent can
/// report exactly which op diverged. Identical in both transcripts, so it can
/// never mask a difference.
const OP_MARK: &[u8] = b"#\n";

unsafe fn replay(im: &Impl, ops: &[Op]) {
    for op in ops {
        fflush(std::ptr::null_mut());
        write(1, OP_MARK.as_ptr() as *const c_void, OP_MARK.len());
        match op {
            Op::Run(v) => (im.run)(*v),
            Op::Driver(s) => (im.driver)(s.as_ptr()),
            Op::DriverNull => (im.driver)(std::ptr::null()),
            Op::DriverRaw(v) => (im.driver)(v.as_ptr() as *const c_char),
        }
    }
    fflush(std::ptr::null_mut());
}

/// Redirect fd 1 to `fd`, run `f`, flush, restore fd 1.
unsafe fn capture_to(fd: c_int, f: impl FnOnce()) {
    fflush(std::ptr::null_mut());
    let saved = dup(1);
    assert!(saved >= 0, "dup(1) failed");
    assert!(dup2(fd, 1) >= 0, "dup2 onto stdout failed");
    f();
    fflush(std::ptr::null_mut());
    dup2(saved, 1);
    close(saved);
}

fn sanitize(label: &str) -> String {
    label
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn wait_status(pid: c_int) -> c_int {
    let mut status: c_int = 0;
    unsafe { waitpid(pid, &mut status, 0) };
    status
}

fn signaled(status: c_int) -> Option<c_int> {
    let low = status & 0x7f;
    if low != 0 && low != 0x7f {
        Some(low)
    } else {
        None
    }
}

fn exit_code(status: c_int) -> Option<c_int> {
    if status & 0x7f == 0 {
        Some((status >> 8) & 0xff)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// The core differential assertion
// ---------------------------------------------------------------------------

/// Replay `ops` against BOTH libraries from pristine state in a forked child and
/// assert the two `stdout` transcripts are byte-for-byte identical.
pub fn assert_same(label: &str, ops: &[Op]) {
    let pair = pair();
    let dir = std::env::temp_dir();
    let tag = format!("drvdiff_{}_{}", std::process::id(), sanitize(label));
    let path_c = dir.join(format!("{}_c.out", tag));
    let path_r = dir.join(format!("{}_r.out", tag));

    // Open both capture files in the PARENT so the child performs no allocation.
    let file_c = std::fs::File::create(&path_c).expect("create C capture file");
    let file_r = std::fs::File::create(&path_r).expect("create Rust capture file");
    let fd_c = file_c.as_raw_fd();
    let fd_r = file_r.as_raw_fd();

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        // ---- child ----
        unsafe {
            capture_to(fd_c, || replay(&pair.c, ops));
            capture_to(fd_r, || replay(&pair.rust, ops));
            _exit(0);
        }
    }

    let status = wait_status(pid);
    if let Some(sig) = signaled(status) {
        panic!(
            "[{}] scenario child was killed by signal {} while replaying {} ops \
             (both implementations are expected to survive valid input)",
            label,
            sig,
            ops.len()
        );
    }
    assert_eq!(
        exit_code(status),
        Some(0),
        "[{}] scenario child exited abnormally (status {:#x})",
        label,
        status
    );

    let out_c = std::fs::read(&path_c).expect("read C capture");
    let out_r = std::fs::read(&path_r).expect("read Rust capture");
    let _ = std::fs::remove_file(&path_c);
    let _ = std::fs::remove_file(&path_r);

    if out_c != out_r {
        panic!("{}", render_diff(label, ops, &out_c, &out_r));
    }

    // Sanity: the transcript must actually contain output, otherwise the test is
    // vacuously "passing" because nothing was captured.
    assert!(
        out_c.len() > ops.len() * OP_MARK.len(),
        "[{}] captured no library output at all ({} bytes for {} ops) — \
         stdout redirection is broken, so this comparison proves nothing",
        label,
        out_c.len(),
        ops.len()
    );
}

/// Replay `ops` against a SINGLE library from pristine state (in a forked child)
/// and return its `stdout` transcript. Used for harness introspection and for
/// asserting the exact expected bytes of the error message.
pub fn capture_one(im: &Impl, ops: &[Op]) -> String {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "drvone_{}_{}_{}.out",
        std::process::id(),
        im.name,
        ops.len()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");
    let fd = file.as_raw_fd();

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        unsafe {
            capture_to(fd, || replay(im, ops));
            _exit(0);
        }
    }
    let status = wait_status(pid);
    assert_eq!(
        exit_code(status),
        Some(0),
        "capture_one child terminated abnormally (status {:#x})",
        status
    );
    let out = std::fs::read(&path).expect("read capture");
    let _ = std::fs::remove_file(&path);
    // Strip the op markers so callers see only library output.
    String::from_utf8_lossy(&out).replace("#\n", "")
}

fn render_diff(label: &str, ops: &[Op], a: &[u8], b: &[u8]) -> String {
    let sa = String::from_utf8_lossy(a);
    let sb = String::from_utf8_lossy(b);
    let la: Vec<&str> = sa.lines().collect();
    let lb: Vec<&str> = sb.lines().collect();

    let mut first = usize::MAX;
    for i in 0..la.len().max(lb.len()) {
        if la.get(i) != lb.get(i) {
            first = i;
            break;
        }
    }

    // Which op does the first differing line belong to? Count markers before it.
    let op_idx = la
        .iter()
        .take(first.min(la.len()))
        .filter(|l| **l == "#")
        .count()
        .saturating_sub(1);

    let mut s = String::new();
    s.push_str(&format!(
        "\n=== DIVERGENCE in scenario [{}] ===\n{} ops replayed; C transcript {} bytes, Rust {} bytes\n",
        label,
        ops.len(),
        a.len(),
        b.len()
    ));
    if let Some(op) = ops.get(op_idx) {
        s.push_str(&format!(
            "first difference at transcript line {} => op #{}: {}\n",
            first, op_idx, op.describe()
        ));
    }
    let lo = first.saturating_sub(6);
    let hi = (first + 7).min(la.len().max(lb.len()));
    s.push_str("\n  line | C                                        | Rust\n");
    for i in lo..hi {
        let x = la.get(i).copied().unwrap_or("<missing>");
        let y = lb.get(i).copied().unwrap_or("<missing>");
        s.push_str(&format!(
            "{}{:5} | {:40} | {}\n",
            if x == y { "  " } else { ">>" },
            i,
            x,
            y
        ));
    }
    s
}

// ---------------------------------------------------------------------------
// Crash-parity helper (for the NULL-pointer row)
// ---------------------------------------------------------------------------

/// How a child process terminated.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Exited(c_int),
    Signaled(c_int),
}

/// Fork a child that performs `op` against `im` and report how it terminated.
/// Used for inputs (NULL) where the C code has no guard and faults.
pub fn outcome_of(im: &Impl, op: &Op) -> Outcome {
    // Send any output to /dev/null so a crashing child does not pollute the
    // test log.
    let devnull = std::fs::File::create("/dev/null").expect("open /dev/null");
    let fd = devnull.as_raw_fd();

    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        unsafe {
            dup2(fd, 1);
            match op {
                Op::Run(v) => (im.run)(*v),
                Op::Driver(s) => (im.driver)(s.as_ptr()),
                Op::DriverNull => (im.driver)(std::ptr::null()),
                Op::DriverRaw(v) => (im.driver)(v.as_ptr() as *const c_char),
            }
            fflush(std::ptr::null_mut());
            _exit(0);
        }
    }
    let status = wait_status(pid);
    match signaled(status) {
        Some(sig) => Outcome::Signaled(sig),
        None => Outcome::Exited(exit_code(status).unwrap_or(-1)),
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) — fixed seeds for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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
    /// Uniform over the FULL `i32` domain (any bit pattern).
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Inclusive range.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}
