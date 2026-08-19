//! Shared differential-testing harness.
//!
//! Both the C shared object (`c_src/build/libdriver.so`) and the Rust shared
//! object (`target/<profile>/libdriver.so`) are loaded with `libloading` and
//! driven exclusively through their exported C symbols, so the `#[no_mangle]`
//! export wrappers are part of what is under test.  No Rust function is ever
//! called directly.
//!
//! The library writes to `stdout` through the C runtime's `printf`, so its output
//! has to be captured at the *file-descriptor* level.  Redirecting fd 1 is
//! process-global, and libtest writes its own progress messages to fd 1 from
//! other threads, so every capture happens in a `fork()`ed child instead:
//!
//! * the child points fd 1 at a temp file, runs a whole *batch* of calls, and
//!   after each call records the current file offset in a second temp file;
//! * the parent slices the output file with those offsets, so it gets the exact
//!   bytes produced by each individual call while paying for only one `fork()`
//!   per batch;
//! * a child that dies (the C code under test contains a deliberate
//!   out-of-bounds stack write) cannot take the test process down, and the
//!   already-produced bytes are still on disk.

#![allow(dead_code)]

use libloading::os::unix::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need (declared directly to avoid an extra dependency)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
    fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
    fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
    static stdout: *mut c_void;
}

#[repr(C)]
struct RLimit {
    rlim_cur: u64,
    rlim_max: u64,
}

const RLIMIT_CORE: c_int = 4;

const IONBF: c_int = 2; // _IONBF
const IOFBF: c_int = 0; // _IOFBF
const SEEK_CUR: c_int = 1;

// dlopen flags: resolve now, and keep the symbols out of the global namespace so
// the two libraries can never satisfy each other's `printLine`/`printIntLine`
// relocations.
const RTLD_NOW: c_int = 0x2;
const RTLD_LOCAL: c_int = 0;

// ---------------------------------------------------------------------------
// Library handles
// ---------------------------------------------------------------------------

/// One loaded `libdriver.so` (either the C or the Rust one) with every exported
/// symbol resolved through `dlsym`.
pub struct Driver {
    pub name: &'static str,
    pub path: PathBuf,
    pub print_line: Symbol<unsafe extern "C" fn(*const c_char)>,
    pub print_int_line: Symbol<unsafe extern "C" fn(c_int)>,
    pub bad: Symbol<unsafe extern "C" fn(c_int)>,
    pub good: Symbol<unsafe extern "C" fn(c_int)>,
    pub driver: Symbol<unsafe extern "C" fn(c_int, c_int)>,
    _lib: Library,
}

impl Driver {
    fn load(name: &'static str, path: PathBuf) -> Driver {
        assert!(
            path.exists(),
            "shared object {} does not exist (build it first)",
            path.display()
        );
        let lib = unsafe { Library::open(Some(&path), RTLD_NOW | RTLD_LOCAL) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        macro_rules! sym {
            ($n:literal) => {
                unsafe { lib.get($n) }.unwrap_or_else(|e| {
                    panic!(
                        "dlsym({}, {}) failed: {e}",
                        path.display(),
                        std::str::from_utf8($n).unwrap()
                    )
                })
            };
        }
        Driver {
            name,
            print_line: sym!(b"printLine\0"),
            print_int_line: sym!(b"printIntLine\0"),
            bad: sym!(b"bad\0"),
            good: sym!(b"good\0"),
            driver: sym!(b"driver\0"),
            path,
            _lib: lib,
        }
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libdriver.so`, built on demand with cmake if absent.
pub fn c_so_path() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let build = manifest_dir().join("c_src/build");
            let so = build.join("libdriver.so");
            if !so.exists() {
                std::fs::create_dir_all(&build).expect("mkdir c_src/build");
                let cfg = std::process::Command::new("cmake")
                    .current_dir(&build)
                    .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
                    .output()
                    .expect("run cmake");
                assert!(cfg.status.success(), "cmake configure failed: {cfg:?}");
                let bld = std::process::Command::new("cmake")
                    .current_dir(&build)
                    .args(["--build", "."])
                    .output()
                    .expect("run cmake --build");
                assert!(bld.status.success(), "cmake build failed: {bld:?}");
            }
            so
        })
        .clone()
}

/// `target/<profile>/libdriver.so`, derived from the test binary's own location
/// (`target/<profile>/deps/<test>-<hash>`).
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir: &Path = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>");
    profile_dir.join("libdriver.so")
}

static C_LIB: OnceLock<Driver> = OnceLock::new();
static RUST_LIB: OnceLock<Driver> = OnceLock::new();

pub fn c_lib() -> &'static Driver {
    C_LIB.get_or_init(|| Driver::load("C", c_so_path()))
}

pub fn rust_lib() -> &'static Driver {
    RUST_LIB.get_or_init(|| Driver::load("Rust", rust_so_path()))
}

// ---------------------------------------------------------------------------
// Operations (the public API surface)
// ---------------------------------------------------------------------------

/// One call into the library's public API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// `printLine(NULL)`
    PrintLineNull,
    /// `printLine(<bytes>)` – NUL terminated by the harness.
    PrintLine(Vec<u8>),
    /// `printLine(buf + off)` – interior pointer into a larger buffer.
    PrintLineRaw(Vec<u8>, usize),
    /// `printIntLine(n)`
    PrintIntLine(c_int),
    /// `bad(data)`
    Bad(c_int),
    /// `good(data)`
    Good(c_int),
    /// `driver(goodData, badData)`
    Driver(c_int, c_int),
}

impl Op {
    pub fn describe(&self) -> String {
        match self {
            Op::PrintLineNull => "printLine(NULL)".to_string(),
            Op::PrintLine(b) => format!("printLine({:?})", String::from_utf8_lossy(b)),
            Op::PrintLineRaw(b, off) => {
                format!("printLine(buf+{off}) buf={:?}", String::from_utf8_lossy(b))
            }
            Op::PrintIntLine(n) => format!("printIntLine({n})"),
            Op::Bad(d) => format!("bad({d})"),
            Op::Good(d) => format!("good({d})"),
            Op::Driver(g, b) => format!("driver({g}, {b})"),
        }
    }

    /// True when this call makes the C code perform its deliberate
    /// out-of-bounds stack store (`bad(data)` with `data >= 10`), i.e. when the
    /// *termination* of the process is undefined behaviour and therefore not
    /// required to match.  The bytes written to stdout are still fully
    /// determined and are always compared.
    pub fn is_ub(&self) -> bool {
        match *self {
            Op::Bad(d) => d >= 10,
            Op::Driver(_, b) => b >= 10,
            _ => false,
        }
    }
}

/// Perform `op` against `d`.  Output goes to fd 1 (redirect it first).
pub fn run_op(d: &Driver, op: &Op) {
    unsafe {
        match op {
            Op::PrintLineNull => (d.print_line)(std::ptr::null()),
            Op::PrintLine(bytes) => {
                let mut z = bytes.clone();
                z.push(0);
                (d.print_line)(z.as_ptr() as *const c_char);
            }
            Op::PrintLineRaw(bytes, off) => {
                (d.print_line)(bytes.as_ptr().add(*off) as *const c_char);
            }
            Op::PrintIntLine(n) => (d.print_int_line)(*n),
            Op::Bad(v) => (d.bad)(*v),
            Op::Good(v) => (d.good)(*v),
            Op::Driver(g, b) => (d.driver)(*g, *b),
        }
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

static COUNTER: Mutex<u64> = Mutex::new(0);

fn temp_path(tag: &str) -> PathBuf {
    let mut c = COUNTER.lock().unwrap();
    *c += 1;
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    dir.join(format!("driver_diff_{}_{}_{}.bin", std::process::id(), tag, *c))
}

/// stdout buffering mode used inside the child.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Buffering {
    /// libc default for a regular file: fully buffered.
    Block,
    /// `setvbuf(stdout, NULL, _IONBF, 0)`.
    Unbuffered,
}

/// How a child terminated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Exit {
    Code(i32),
    Signal(i32),
}

/// Result of running a batch of calls against one library in one child process.
#[derive(Clone, Debug)]
pub struct BatchResult {
    /// Bytes produced by each call that ran to completion.
    pub outputs: Vec<Vec<u8>>,
    /// Bytes produced after the last completed call (non-empty only if the child
    /// died in the middle of a call).
    pub partial: Vec<u8>,
    /// The whole stdout stream.
    pub full: Vec<u8>,
    pub exit: Exit,
}

impl BatchResult {
    pub fn completed(&self) -> usize {
        self.outputs.len()
    }
}

/// Run every op in `ops` against `d` inside a forked child, capturing the bytes
/// each individual call writes to fd 1.
pub fn run_batch(d: &Driver, ops: &[Op], buffering: Buffering) -> BatchResult {
    let pairs: Vec<(&Driver, &Op)> = ops.iter().map(|op| (d, op)).collect();
    run_batch_pairs(&pairs, buffering)
}

/// Like [`run_batch`], but each call chooses its own library, so both `.so`s can
/// be exercised inside a single process and a single stdout stream.
pub fn run_batch_pairs(ops: &[(&Driver, &Op)], buffering: Buffering) -> BatchResult {
    let out_path = temp_path("out");
    let off_path = temp_path("off");
    let out_file = std::fs::File::create(&out_path).expect("create output temp file");
    let off_file = std::fs::File::create(&off_path).expect("create offsets temp file");
    let (out_fd, off_fd) = (out_file.as_raw_fd(), off_file.as_raw_fd());

    unsafe { fflush(std::ptr::null_mut()) };
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        // ---------------- child ----------------
        unsafe {
            // The C code under test deliberately smashes its stack frame for
            // `data >= 10`, so children are *expected* to die from SIGSEGV.
            // Suppress core dumps: they are useless here and writing them makes
            // the UB rows take minutes instead of seconds.
            let no_core = RLimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            setrlimit(RLIMIT_CORE, &no_core);
            dup2(out_fd, 1);
            match buffering {
                Buffering::Block => {
                    setvbuf(stdout, std::ptr::null_mut(), IOFBF, 4096);
                }
                Buffering::Unbuffered => {
                    setvbuf(stdout, std::ptr::null_mut(), IONBF, 0);
                }
            }
            for (d, op) in ops {
                run_op(d, op);
                // Make the bytes of this call visible in the file, then record
                // where it ended.
                fflush(std::ptr::null_mut());
                let pos = lseek(1, 0, SEEK_CUR);
                let bytes = pos.to_le_bytes();
                write(off_fd, bytes.as_ptr() as *const c_void, bytes.len());
            }
            _exit(0);
        }
    }

    // ---------------- parent ----------------
    let mut status: c_int = 0;
    let r = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(r, pid, "waitpid failed");
    drop(out_file);
    drop(off_file);
    let full = std::fs::read(&out_path).expect("read output temp file");
    let offs_raw = std::fs::read(&off_path).expect("read offsets temp file");
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&off_path);

    let mut outputs = Vec::new();
    let mut prev = 0usize;
    for chunk in offs_raw.chunks_exact(8) {
        let end = i64::from_le_bytes(chunk.try_into().unwrap()).max(0) as usize;
        let end = end.min(full.len());
        outputs.push(full[prev.min(end)..end].to_vec());
        prev = end;
    }
    let partial = full[prev.min(full.len())..].to_vec();
    let exit = if status & 0x7f == 0 {
        Exit::Code((status >> 8) & 0xff)
    } else {
        Exit::Signal(status & 0x7f)
    };
    BatchResult {
        outputs,
        partial,
        full,
        exit,
    }
}

/// Run one *undefined-behaviour* call (`bad(data)` / `driver(_, data)` with
/// `data >= 10`) in a child dedicated to it.
///
/// The C store `buffer[data] = 1` writes above `bad`'s frame, i.e. into the
/// caller's frame — which, in this harness, is the harness itself.  So this child
/// is deliberately minimal:
///
/// * `stdout` is unbuffered, so every byte reaches the file before any crash and
///   no bookkeeping is needed after the call;
/// * a large sacrificial stack cushion sits directly above the callee's frame, so
///   moderate overflows land in dead space instead of in the harness's state;
/// * the library is called directly from the child's own frame and the child then
///   calls `_exit` without returning, so a smashed return address in the harness
///   frame cannot turn into a spurious crash or corrupt the captured stream.
///
/// Only the whole stdout stream (`BatchResult::full`) is meaningful here.
pub fn run_ub(d: &Driver, op: &Op) -> BatchResult {
    let out_path = temp_path("ub");
    let out_file = std::fs::File::create(&out_path).expect("create output temp file");
    let out_fd = out_file.as_raw_fd();

    unsafe { fflush(std::ptr::null_mut()) };
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        // ---------------- child ----------------
        unsafe {
            let no_core = RLimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            setrlimit(RLIMIT_CORE, &no_core);
            dup2(out_fd, 1);
            setvbuf(stdout, std::ptr::null_mut(), IONBF, 0);
            let mut cushion = [0u8; 1 << 16];
            std::hint::black_box(cushion.as_mut_ptr());
            match *op {
                Op::Bad(v) => (d.bad)(v),
                Op::Driver(g, b) => (d.driver)(g, b),
                Op::Good(v) => (d.good)(v),
                _ => {
                    // not an out-of-bounds capable entry point
                    write(2, c"bad op for run_ub\n".as_ptr() as *const c_void, 18);
                    _exit(9);
                }
            }
            std::hint::black_box(cushion.as_mut_ptr());
            _exit(0);
        }
    }

    // ---------------- parent ----------------
    let mut status: c_int = 0;
    let r = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(r, pid, "waitpid failed");
    drop(out_file);
    let full = std::fs::read(&out_path).expect("read output temp file");
    let _ = std::fs::remove_file(&out_path);
    let exit = if status & 0x7f == 0 {
        Exit::Code((status >> 8) & 0xff)
    } else {
        Exit::Signal(status & 0x7f)
    };
    assert_ne!(exit, Exit::Code(9), "run_ub used with an unsupported op");
    BatchResult {
        outputs: Vec::new(),
        partial: full.clone(),
        full,
        exit,
    }
}

/// Per-call outputs of one library, requiring a clean run (used for the
/// model-based cross-checks).  Panics if the child did not finish every call.
pub fn outputs(d: &Driver, ops: &[Op]) -> Vec<Vec<u8>> {
    let r = run_batch(d, ops, Buffering::Block);
    assert_eq!(
        r.exit,
        Exit::Code(0),
        "{} lib terminated abnormally ({:?}) while running {:?}",
        d.name,
        r.exit,
        ops.iter().map(Op::describe).collect::<Vec<_>>()
    );
    assert_eq!(
        r.completed(),
        ops.len(),
        "{} lib only completed {}/{} calls",
        d.name,
        r.completed(),
        ops.len()
    );
    r.outputs
}

/// Output of a single call against one library (clean run required).
pub fn output(d: &Driver, op: &Op) -> Vec<u8> {
    outputs(d, std::slice::from_ref(op)).pop().unwrap()
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// Run `ops` against both libraries (one child each) and assert that every call
/// produced byte-identical output and that both children terminated identically.
///
/// `ops` must not contain UB calls (see [`Op::is_ub`]); those go through
/// [`assert_same_stdout_ub`].
pub fn assert_same_batch(ops: &[Op]) {
    assert_same_batch_buffered(ops, Buffering::Block);
}

pub fn assert_same_batch_buffered(ops: &[Op], buffering: Buffering) {
    for op in ops {
        assert!(
            !op.is_ub(),
            "{} is an undefined-behaviour call; use assert_same_stdout_ub",
            op.describe()
        );
    }
    let c = run_batch(c_lib(), ops, buffering);
    let r = run_batch(rust_lib(), ops, buffering);
    let n = c.outputs.len().min(r.outputs.len());
    for i in 0..n {
        assert_eq!(
            c.outputs[i],
            r.outputs[i],
            "output mismatch for {}\n  C   : \"{}\"\n  Rust: \"{}\"",
            ops[i].describe(),
            show(&c.outputs[i]),
            show(&r.outputs[i])
        );
    }
    assert_eq!(
        c.exit,
        Exit::Code(0),
        "C lib terminated abnormally after {} of {} calls (last: {})",
        c.completed(),
        ops.len(),
        ops.get(c.completed()).map(Op::describe).unwrap_or_default()
    );
    assert_eq!(
        r.exit,
        Exit::Code(0),
        "Rust lib terminated abnormally after {} of {} calls (last: {})",
        r.completed(),
        ops.len(),
        ops.get(r.completed()).map(Op::describe).unwrap_or_default()
    );
    assert_eq!(c.completed(), ops.len(), "C lib did not run every call");
    assert_eq!(r.completed(), ops.len(), "Rust lib did not run every call");
    assert_eq!(
        c.full,
        r.full,
        "whole-transcript mismatch\n  C   : \"{}\"\n  Rust: \"{}\"",
        show(&c.full),
        show(&r.full)
    );
}

/// Single non-UB call.
pub fn assert_same(op: &Op) {
    assert_same_batch(std::slice::from_ref(op));
}

/// Compare one UB call (`bad(data)` with `data >= 10`, directly or through
/// `driver`).
///
/// The C store `buffer[data] = 1` lands outside `buffer`, so from that point on
/// the C source's meaning is only "write 4 bytes at `&buffer[0] + 4*data` on the
/// stack".  Which slot that is — padding, a saved register, a return address, or
/// an unmapped page — is decided by the *compiler's* frame layout, so the point
/// at which the smashed frame kills the process is not a property of the C
/// program and cannot be required to match.  What *is* fully determined is the
/// byte stream itself (the ten in-bounds elements are never touched by the
/// store), so this assertion requires:
///
/// * both processes survived  ⇒ the streams must be **identical**, and
/// * one process was killed   ⇒ the shorter stream must be a **prefix** of the
///   longer one (identical output up to the moment the frame smash took one of
///   them down).
pub fn assert_same_stdout_ub(op: &Op) {
    let c = run_ub(c_lib(), op);
    let r = run_ub(rust_lib(), op);
    let both_survived = c.exit == Exit::Code(0) && r.exit == Exit::Code(0);
    if both_survived {
        assert_eq!(
            c.full,
            r.full,
            "output mismatch for {} (UB row, both processes survived)\n  C   : \"{}\"\n  Rust: \"{}\"",
            op.describe(),
            show(&c.full),
            show(&r.full)
        );
        return;
    }
    let (short, long) = if c.full.len() <= r.full.len() {
        (&c.full, &r.full)
    } else {
        (&r.full, &c.full)
    };
    assert!(
        long.starts_with(short.as_slice()),
        "output mismatch for {} (UB row)\n  C   ({:?}): \"{}\"\n  Rust ({:?}): \"{}\"",
        op.describe(),
        c.exit,
        show(&c.full),
        r.exit,
        show(&r.full)
    );
    println!(
        "note: {} — the deliberate out-of-bounds store smashed the frame; \
         C {:?} produced {} bytes, Rust {:?} produced {} bytes, \
         and the shorter stream is a prefix of the longer one",
        op.describe(),
        c.exit,
        c.full.len(),
        r.exit,
        r.full.len()
    );
}

/// Whole stdout stream of one UB call against one library.
pub fn ub_stream(d: &Driver, op: &Op) -> Vec<u8> {
    run_ub(d, op).full
}

// ---------------------------------------------------------------------------
// Expected-output model taken from the C source
// ---------------------------------------------------------------------------

/// The ten `printIntLine` lines printed for a `buffer[10]` in which
/// `buffer[idx] = 1` (or nothing, when `idx` is out of bounds ⇒ `None`).
pub fn ten_lines(idx: Option<usize>) -> Vec<u8> {
    let mut s = Vec::new();
    for i in 0..10usize {
        s.extend_from_slice(if Some(i) == idx { b"1\n" } else { b"0\n" });
    }
    s
}

pub const ERR_NEGATIVE: &[u8] = b"ERROR: Array index is negative.\n";
pub const ERR_OOB: &[u8] = b"ERROR: Array index is out-of-bounds\n";

/// Deterministic PRNG (splitmix64) so every randomised test is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
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
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn next_byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
}
