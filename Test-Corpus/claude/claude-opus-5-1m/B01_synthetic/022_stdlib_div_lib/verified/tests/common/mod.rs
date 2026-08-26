//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as *shared libraries* with `libloading` and
//! called only through their exported `driver` symbol, so the `#[no_mangle]`
//! export wrapper is part of what is under test. No Rust function is ever
//! called directly.
//!
//! # Why every comparison runs in a child process
//!
//! `driver`'s entire observable behaviour is the bytes it writes to the
//! process-wide `stdout` `FILE` via libc `printf`. Capturing that means
//! redirecting file descriptor 1, which is a *process-global* resource: doing it
//! inside the test process races against libtest's own progress output from
//! other threads (and against any other test). So instead, each comparison is
//! executed by re-`exec`ing this test binary as a **worker** child whose fd 1 is
//! a file nobody else writes to.
//!
//! That also makes the harness strictly better than an in-process one:
//!
//! * the library is driven from a clean process, exactly as a real consumer
//!   would, including `stdout` buffering across a long stream of calls;
//! * a batch of many calls costs only *one* child, so rows can use tens of
//!   thousands of randomized inputs;
//! * calls that die from `SIGFPE` are observable instead of taking the runner
//!   down with them.

#![allow(dead_code)]

use std::ffi::{c_int, c_void};
use std::fmt::Write as _;
use std::io::Write as _;
use std::os::fd::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// `void driver(int x, int y);`
pub type DriverFn = unsafe extern "C" fn(c_int, c_int);

unsafe extern "C" {
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open output stream, which is how the
    /// worker forces libc's `stdout` buffer out to the redirected fd.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// SIGFPE — how `driver` "reports" the two conditions in `ERRORS.md`.
pub const SIGFPE: i32 = 8;

// ---------------------------------------------------------------------------
// Which implementation to call
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    C,
    Rust,
    /// The C `driver` symbol, re-typed as `fn(i64, i64)` so the caller can leave
    /// garbage in the upper halves of the argument registers.
    C64,
    /// The Rust `driver` symbol, re-typed the same way.
    Rust64,
}

impl Side {
    pub fn tag(self) -> &'static str {
        match self {
            Side::C => "c",
            Side::Rust => "rust",
            Side::C64 => "c64",
            Side::Rust64 => "rust64",
        }
    }

    fn parse(s: &str) -> Side {
        match s {
            "c" => Side::C,
            "rust" => Side::Rust,
            "c64" => Side::C64,
            "rust64" => Side::Rust64,
            other => panic!("unknown side {other:?}"),
        }
    }

    /// Whether this side calls through the 32-bit `int` signature.
    fn is_native(self) -> bool {
        matches!(self, Side::C | Side::Rust)
    }
}

/// One `driver(x, y)` invocation against one implementation.
///
/// `x`/`y` are stored as `i64` so the `*64` sides can carry full 64-bit
/// register values; native sides always hold values that fit in `c_int`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Op {
    pub side: Side,
    pub x: i64,
    pub y: i64,
}

pub fn op(side: Side, x: c_int, y: c_int) -> Op {
    Op {
        side,
        x: x as i64,
        y: y as i64,
    }
}

/// An op carrying raw 64-bit register values (for the `*64` sides).
pub fn op_raw(side: Side, x: i64, y: i64) -> Op {
    Op { side, x, y }
}

/// Builds a batch that runs every pair against a single implementation.
pub fn ops_for(side: Side, pairs: &[(c_int, c_int)]) -> Vec<Op> {
    pairs.iter().map(|&(x, y)| op(side, x, y)).collect()
}

// ---------------------------------------------------------------------------
// Locating the two shared libraries
// ---------------------------------------------------------------------------

/// `c_src/build/libdriver.so`, built by CMake.
pub fn c_so_path() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// `target/<profile>/libdriver.so`, built by `cargo build`.
///
/// Derived from the running test executable (`target/<profile>/deps/<exe>`) so
/// it follows `--release`, `CARGO_TARGET_DIR` and custom profiles.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>");
    let p = profile_dir.join("libdriver.so");
    assert!(
        p.is_file(),
        "Rust shared library not found at {}\nBuild it with: cargo build",
        p.display()
    );
    p
}

/// Loads both `.so`s once and returns their exported `driver` symbols.
///
/// The `Library` handles are leaked on purpose: the returned function pointers
/// must stay valid for the rest of the process's life.
pub fn drivers() -> (DriverFn, DriverFn) {
    static FNS: OnceLock<(DriverFn, DriverFn)> = OnceLock::new();
    *FNS.get_or_init(|| unsafe {
        let c_lib: &'static Library =
            Box::leak(Box::new(Library::new(c_so_path()).expect("dlopen C .so")));
        let r_lib: &'static Library =
            Box::leak(Box::new(Library::new(rust_so_path()).expect("dlopen Rust .so")));

        let c_sym: Symbol<'static, DriverFn> =
            c_lib.get(b"driver\0").expect("C .so exports `driver`");
        let r_sym: Symbol<'static, DriverFn> =
            r_lib.get(b"driver\0").expect("Rust .so exports `driver`");

        (*c_sym, *r_sym)
    })
}

/// The exported `driver` symbol of the requested implementation, with its true
/// `fn(int, int)` signature.
pub fn driver_for(side: Side) -> DriverFn {
    let (c, r) = drivers();
    match side {
        Side::C | Side::C64 => c,
        Side::Rust | Side::Rust64 => r,
    }
}

// ---------------------------------------------------------------------------
// The worker child
// ---------------------------------------------------------------------------

const ENV_OPS: &str = "DIFFTEST_OPS";
const ENV_OUT: &str = "DIFFTEST_OUT";
const ENV_UNBUFFERED: &str = "DIFFTEST_UNBUFFERED";
/// Name of the `#[test]` in every test file that acts as the child entry point.
pub const WORKER: &str = "difftest_worker";

fn unique_tmp_path(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver-difftest-{}-{tag}-{n}",
        std::process::id()
    ))
}

/// How a worker terminated, plus everything `driver` wrote to its `stdout`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Outcome {
    /// `Some(signum)` when killed by a signal — e.g. `Some(8)` for `SIGFPE`.
    pub signal: Option<i32>,
    /// `Some(code)` when it exited normally.
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
}

impl Outcome {
    /// The `\n`-terminated lines of captured output, newlines stripped.
    pub fn lines(&self) -> Vec<&[u8]> {
        if self.stdout.is_empty() {
            return Vec::new();
        }
        assert!(
            self.stdout.ends_with(b"\n"),
            "captured output is not newline-terminated: {:?}",
            String::from_utf8_lossy(&self.stdout)
        );
        self.stdout[..self.stdout.len() - 1]
            .split(|&b| b == b'\n')
            .collect()
    }
}

/// Runs `ops` in order inside one freshly `exec`ed child process.
///
/// `unbuffered` makes the worker flush after every call, so output produced
/// *before* a trap is still observable; otherwise libc's normal full buffering
/// applies (which is what the buffering-parity rows want to exercise).
pub fn run_ops_with(ops: &[Op], unbuffered: bool) -> Outcome {
    assert!(!ops.is_empty(), "run_ops called with no operations");

    let exe = std::env::current_exe().expect("current_exe");
    let ops_path = unique_tmp_path("ops");
    let out_path = unique_tmp_path("out");

    let mut script = String::with_capacity(ops.len() * 16);
    for o in ops {
        writeln!(script, "{} {} {}", o.side.tag(), o.x, o.y).unwrap();
    }
    std::fs::write(&ops_path, script).expect("write ops file");

    let mut cmd = Command::new(&exe);
    cmd.args([WORKER, "--exact", "--test-threads=1", "--nocapture"])
        .env(ENV_OPS, &ops_path)
        .env(ENV_OUT, &out_path)
        // The child re-runs the libtest harness; keep its banner out of the way.
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if unbuffered {
        cmd.env(ENV_UNBUFFERED, "1");
    }

    let status = cmd.status().expect("spawn difftest worker");
    let stdout = std::fs::read(&out_path).unwrap_or_default();
    let _ = std::fs::remove_file(&ops_path);
    let _ = std::fs::remove_file(&out_path);

    Outcome {
        signal: status.signal(),
        code: status.code(),
        stdout,
    }
}

/// `run_ops_with(ops, false)` — normal libc buffering.
pub fn run_ops(ops: &[Op]) -> Outcome {
    run_ops_with(ops, false)
}

/// The child-side body. Every test file defines a `difftest_worker` test that
/// delegates here; it is inert unless the environment says otherwise.
pub fn worker_body() {
    let Ok(ops_path) = std::env::var(ENV_OPS) else {
        return; // ordinary test run: nothing to do
    };
    let out_path = std::env::var(ENV_OUT).expect("DIFFTEST_OUT");
    let unbuffered = std::env::var(ENV_UNBUFFERED).is_ok();

    let script = std::fs::read_to_string(&ops_path).expect("read ops file");
    let ops: Vec<Op> = script
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut it = l.split_whitespace();
            let side = Side::parse(it.next().expect("side"));
            let x: i64 = it.next().expect("x").parse().expect("parse x");
            let y: i64 = it.next().expect("y").parse().expect("parse y");
            op_raw(side, x, y)
        })
        .collect();

    // Resolve both symbols *before* redirecting, so dlopen diagnostics are not
    // swallowed and so no loader output can land in the captured file.
    let (c, r) = drivers();

    let file = std::fs::File::create(&out_path).expect("create worker out file");
    std::io::stdout().flush().ok();
    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    // The exact same exported symbols, re-typed with 64-bit parameters so the
    // caller can leave garbage in the upper halves of RDI/RSI.
    type Driver64 = unsafe extern "C" fn(i64, i64);
    let c64: Driver64 = unsafe { std::mem::transmute::<DriverFn, Driver64>(c) };
    let r64: Driver64 = unsafe { std::mem::transmute::<DriverFn, Driver64>(r) };

    for o in &ops {
        // If any of these traps, the process dies here and the parent sees the
        // signal.
        match o.side {
            Side::C => unsafe { c(o.x as c_int, o.y as c_int) },
            Side::Rust => unsafe { r(o.x as c_int, o.y as c_int) },
            Side::C64 => unsafe { c64(o.x, o.y) },
            Side::Rust64 => unsafe { r64(o.x, o.y) },
        }
        if unbuffered {
            unsafe { fflush(std::ptr::null_mut()) };
        }
    }

    unsafe { fflush(std::ptr::null_mut()) };
    // Reached only when no call trapped.
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

/// Runs every pair against C, then against Rust, and asserts the two output
/// streams are byte-identical — reporting the offending pair on mismatch.
///
/// Panics if `pairs` is empty, so a silently-empty generator cannot pass a row.
pub fn assert_same_all(label: &str, pairs: &[(c_int, c_int)]) {
    assert!(!pairs.is_empty(), "{label}: no cases were generated");
    for &(x, y) in pairs {
        assert!(
            !traps(x, y),
            "{label}: ({x}, {y}) is a trapping pair; route it through assert_same_trap"
        );
    }

    let c = run_ops(&ops_for(Side::C, pairs));
    let r = run_ops(&ops_for(Side::Rust, pairs));

    assert_eq!(c.signal, None, "{label}: C worker died with {:?}", c.signal);
    assert_eq!(r.signal, None, "{label}: Rust worker died with {:?}", r.signal);
    assert_eq!(c.code, Some(0), "{label}: C worker exit code");
    assert_eq!(r.code, Some(0), "{label}: Rust worker exit code");

    let c_lines = c.lines();
    let r_lines = r.lines();
    assert_eq!(
        c_lines.len(),
        pairs.len(),
        "{label}: C produced {} lines for {} calls",
        c_lines.len(),
        pairs.len()
    );
    assert_eq!(
        r_lines.len(),
        pairs.len(),
        "{label}: Rust produced {} lines for {} calls",
        r_lines.len(),
        pairs.len()
    );

    for (i, (&(x, y), (cl, rl))) in pairs
        .iter()
        .zip(c_lines.iter().zip(r_lines.iter()))
        .enumerate()
    {
        assert_eq!(
            cl,
            rl,
            "{label}: driver({x}, {y}) (call #{i}) diverged\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(cl),
            String::from_utf8_lossy(rl)
        );
        // Guards against an "equal because both were empty/garbage" pass.
        assert!(
            cl.starts_with(b"quotient: "),
            "{label}: driver({x}, {y}) produced unexpected output {:?}",
            String::from_utf8_lossy(cl)
        );
    }

    eprintln!("{label}: {} case(s) matched byte-for-byte", pairs.len());
}

/// Single-pair convenience wrapper around [`assert_same_all`].
pub fn assert_same(x: c_int, y: c_int) {
    assert_same_all(&format!("driver({x}, {y})"), &[(x, y)]);
}

/// Runs one trapping pair on each side and asserts they die identically: same
/// signal, same exit code, same bytes emitted before the trap.
pub fn assert_same_trap(x: c_int, y: c_int) -> Outcome {
    let c = run_ops_with(&[op(Side::C, x, y)], true);
    let r = run_ops_with(&[op(Side::Rust, x, y)], true);

    assert_eq!(
        c.signal, r.signal,
        "driver({x}, {y}): C died with signal {:?}, Rust with {:?}",
        c.signal, r.signal
    );
    assert_eq!(
        c.code, r.code,
        "driver({x}, {y}): C exit code {:?}, Rust {:?}",
        c.code, r.code
    );
    assert_eq!(
        c.stdout,
        r.stdout,
        "driver({x}, {y}): pre-trap output differs\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.signal,
        Some(SIGFPE),
        "driver({x}, {y}) should die with SIGFPE (8), got signal={:?} code={:?}",
        c.signal,
        c.code
    );
    assert!(
        c.stdout.is_empty(),
        "driver({x}, {y}) traps before printf, so no output is expected, got {:?}",
        String::from_utf8_lossy(&c.stdout)
    );
    c
}

/// Parses `quotient: <q>, remainder: <r>` back out of one captured line.
pub fn parse_line(line: &[u8]) -> (i64, i64) {
    let s = std::str::from_utf8(line).expect("utf8 output");
    let s = s.strip_prefix("quotient: ").expect("`quotient: ` prefix");
    let (q, r) = s.split_once(", remainder: ").expect("`, remainder: ` separator");
    (q.parse().expect("quot"), r.parse().expect("rem"))
}

/// C output for a batch of pairs, as parsed `(quot, rem)` pairs. Used by rows
/// that additionally assert a documented invariant of glibc's `div(3)`.
pub fn c_results(pairs: &[(c_int, c_int)]) -> Vec<(i64, i64)> {
    let c = run_ops(&ops_for(Side::C, pairs));
    assert_eq!(c.signal, None);
    let lines = c.lines();
    assert_eq!(lines.len(), pairs.len());
    lines.iter().map(|l| parse_line(l)).collect()
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed → reproducible property-style testing)
// ---------------------------------------------------------------------------

/// SplitMix64. Self-contained so the tests need no extra dependency.
pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform over all 2^32 bit patterns, i.e. the whole `int` range.
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    /// Uniform in `[lo, hi]`.
    pub fn in_range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    /// A divisor that never triggers `ERRORS.md` row 1.
    pub fn nonzero_i32(&mut self) -> i32 {
        loop {
            let v = self.next_i32();
            if v != 0 {
                return v;
            }
        }
    }
}

/// The two argument pairs that trap (`ERRORS.md`): `y == 0`, and `INT_MIN / -1`.
pub fn traps(x: c_int, y: c_int) -> bool {
    y == 0 || (x == i32::MIN && y == -1)
}

/// The boundary values crossed with each other by `CONFIGS.md` row 17.
pub const BOUNDARIES: [c_int; 7] = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
