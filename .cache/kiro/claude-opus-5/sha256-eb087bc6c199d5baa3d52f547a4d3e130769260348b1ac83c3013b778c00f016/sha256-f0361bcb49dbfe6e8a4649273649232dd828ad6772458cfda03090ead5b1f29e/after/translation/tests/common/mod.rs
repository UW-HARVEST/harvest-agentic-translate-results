//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls only their exported
//! `driver` symbol — the Rust implementation is never called directly, so the
//! `#[no_mangle] extern "C"` wrapper and the C ABI are part of what is tested.
//!
//! Output is captured by temporarily pointing file descriptor 1 at an anonymous
//! `memfd`. Both libraries write through the *same* process-wide glibc `stdout`
//! `FILE` object, so the capture is exactly what an external consumer would see.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need (declared here so the harness needs no `libc` dependency)
// ---------------------------------------------------------------------------

extern "C" {
    static mut stdout: *mut c_void;
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn memfd_create(name: *const c_char, flags: c_uint) -> c_int;
    fn open(path: *const c_char, flags: c_int, mode: c_uint) -> c_int;
    fn unlink(path: *const c_char) -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn fesetround(mode: c_int) -> c_int;
    fn fegetround() -> c_int;
}

const SEEK_SET: c_int = 0;
/// `LC_NUMERIC` on glibc/Linux.
pub const LC_NUMERIC: c_int = 1;
/// `LC_ALL` on glibc/Linux.
pub const LC_ALL: c_int = 6;

fn stdout_file() -> *mut c_void {
    unsafe { std::ptr::addr_of!(stdout).read() }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libdriver.so`, built by CMake.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with \
         `cd c_src && mkdir -p build && cd build && \
          cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`"
    );
    p
}

/// The Rust `cdylib`.
///
/// `cargo test` does **not** build the `cdylib` artifact (it only needs the
/// lib's metadata), so `cargo build` / `cargo build --release` must have run.
/// `scripts/run_tests.sh` does that. Search order: `DRIVER_RUST_SO`, the test
/// executable's own profile dir, then `target/release`, then `target/debug`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // current_exe() is .../target/<profile>/deps/<testname>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir").to_path_buf();
    let profile = deps.parent().expect("profile dir").to_path_buf();
    let target = profile.parent().expect("target dir").to_path_buf();
    let candidates = [
        deps.join("libdriver.so"),
        profile.join("libdriver.so"),
        target.join("release/libdriver.so"),
        target.join("debug/libdriver.so"),
    ];
    for p in &candidates {
        if p.exists() {
            return p.clone();
        }
    }
    panic!(
        "Rust cdylib libdriver.so not found (looked in {candidates:?}); \
         run `cargo build --release` first or set DRIVER_RUST_SO"
    );
}

// ---------------------------------------------------------------------------
// The loaded pair
// ---------------------------------------------------------------------------

type DriverFn = unsafe extern "C" fn(f64);

pub struct Pair {
    _c_lib: Library,
    _r_lib: Library,
    pub c_driver: DriverFn,
    pub r_driver: DriverFn,
}

// Raw `extern "C"` fn pointers and `Library` handles are safe to share; the
// libraries stay loaded for the lifetime of the process.
unsafe impl Sync for Pair {}
unsafe impl Send for Pair {}

static PAIR: OnceLock<Pair> = OnceLock::new();
/// Guards the process-global fd-1 redirection used by `capture`.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| unsafe {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        let c_lib = Library::new(&c_path).unwrap_or_else(|e| panic!("dlopen {c_path:?}: {e}"));
        let r_lib = Library::new(&r_path).unwrap_or_else(|e| panic!("dlopen {r_path:?}: {e}"));
        let c_driver = *c_lib
            .get::<DriverFn>(b"driver\0")
            .unwrap_or_else(|e| panic!("`driver` missing from C .so {c_path:?}: {e}"));
        let r_driver = *r_lib
            .get::<DriverFn>(b"driver\0")
            .unwrap_or_else(|e| panic!("`driver` missing from Rust .so {r_path:?}: {e}"));
        Pair {
            _c_lib: c_lib,
            _r_lib: r_lib,
            c_driver,
            r_driver,
        }
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

fn make_scratch_fd() -> c_int {
    let name = b"driver-capture\0";
    let fd = unsafe { memfd_create(name.as_ptr() as *const c_char, 0) };
    if fd >= 0 {
        return fd;
    }
    // Fallback for kernels/libcs without memfd_create: an unlinked temp file.
    let path = format!(
        "/tmp/driver-capture-{}-{:?}\0",
        std::process::id(),
        std::thread::current().id()
    );
    // O_RDWR | O_CREAT | O_TRUNC
    let fd = unsafe { open(path.as_ptr() as *const c_char, 0o2 | 0o100 | 0o1000, 0o600) };
    assert!(fd >= 0, "cannot create scratch file for stdout capture");
    unsafe { unlink(path.as_ptr() as *const c_char) };
    fd
}

/// Runs `f` with file descriptor 1 pointed at a scratch file and returns the
/// bytes it wrote through glibc's `stdout`.
///
/// fd 1 is process-global, so this must not overlap with any other writer.
/// `CAPTURE_LOCK` serializes concurrent callers and `RUST_TEST_THREADS=1`
/// (see `.cargo/config.toml`) keeps libtest's own progress lines out of the
/// window; the explicit flush of Rust's `std` stdout pushes libtest's pending,
/// newline-less `test <name> ... ` prefix to the real stdout first.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = std::io::Write::flush(&mut std::io::stdout());
    unsafe {
        let out = stdout_file();
        // Push anything already buffered to the real stdout so it is not
        // mistaken for output of `f`.
        fflush(out);
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let scratch = make_scratch_fd();
        assert!(dup2(scratch, 1) >= 0, "dup2 onto fd 1 failed");

        f();

        fflush(out);
        assert!(dup2(saved, 1) >= 0, "dup2 restoring fd 1 failed");
        close(saved);

        lseek(scratch, 0, SEEK_SET);
        let mut buf = Vec::new();
        let mut chunk = [0u8; 65536];
        loop {
            let n = read(scratch, chunk.as_mut_ptr() as *mut c_void, chunk.len());
            if n <= 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n as usize]);
        }
        close(scratch);
        buf
    }
}

/// Splits captured output into one `String` per `\n`-terminated line.
fn lines(buf: &[u8]) -> Vec<String> {
    let s = String::from_utf8_lossy(buf);
    let mut v: Vec<String> = s.split('\n').map(|x| x.to_string()).collect();
    // A well-formed capture always ends with "\n", producing a trailing "".
    if v.last().map(|x| x.is_empty()).unwrap_or(false) {
        v.pop();
    }
    v
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

/// One divergence: the input bit pattern, the C line, the Rust line.
#[derive(Debug, Clone)]
pub struct Divergence {
    pub bits: u64,
    pub value: f64,
    pub c: String,
    pub rust: String,
}

impl std::fmt::Display for Divergence {
    fn fmt(&self, w: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            w,
            "bits=0x{:016x} (value {:e})\n      C    = {:?}\n      Rust = {:?}",
            self.bits, self.value, self.c, self.rust
        )
    }
}

/// Calls both `.so` exports on every value in `bits` and returns the mismatches.
///
/// Each side runs in one capture, so this also exercises the composed output
/// stream (many calls into one buffered `FILE`), not just isolated calls.
pub fn compare_batch(bits: &[u64]) -> Vec<Divergence> {
    let p = pair();
    let c_out = capture(|| {
        for &b in bits {
            unsafe { (p.c_driver)(f64::from_bits(b)) }
        }
    });
    let r_out = capture(|| {
        for &b in bits {
            unsafe { (p.r_driver)(f64::from_bits(b)) }
        }
    });

    let cl = lines(&c_out);
    let rl = lines(&r_out);
    assert_eq!(
        cl.len(),
        bits.len(),
        "C produced {} lines for {} inputs",
        cl.len(),
        bits.len()
    );
    assert_eq!(
        rl.len(),
        bits.len(),
        "Rust produced {} lines for {} inputs (raw: {:?})",
        rl.len(),
        bits.len(),
        String::from_utf8_lossy(&r_out).chars().take(400).collect::<String>()
    );

    let mut out = Vec::new();
    for (i, &b) in bits.iter().enumerate() {
        if cl[i] != rl[i] {
            out.push(Divergence {
                bits: b,
                value: f64::from_bits(b),
                c: cl[i].clone(),
                rust: rl[i].clone(),
            });
        }
    }
    out
}

const CHUNK: usize = 2048;

/// Runs a `CONFIGS.md` row: compares every value and panics with a bounded
/// report if any diverge.
pub fn check_row(row: &str, bits: &[u64]) {
    assert!(!bits.is_empty(), "row {row}: empty input set");
    let mut divs: Vec<Divergence> = Vec::new();
    for chunk in bits.chunks(CHUNK) {
        divs.extend(compare_batch(chunk));
        if divs.len() > 40 {
            break;
        }
    }
    if !divs.is_empty() {
        let shown: Vec<String> = divs.iter().take(20).map(|d| format!("  {d}")).collect();
        panic!(
            "{row}: {} of {} inputs diverge (showing up to 20):\n{}",
            divs.len(),
            bits.len(),
            shown.join("\n")
        );
    }
    eprintln!("{row}: OK ({} inputs)", bits.len());
}

/// Compares a single value and returns the two lines (for error-path tests that
/// want to assert on the exact text as well as on equality).
pub fn check_one(row: &str, bits: u64) -> String {
    let divs = compare_batch(&[bits]);
    if let Some(d) = divs.first() {
        panic!("{row}: diverges\n  {d}");
    }
    let p = pair();
    let out = capture(|| unsafe { (p.c_driver)(f64::from_bits(bits)) });
    String::from_utf8_lossy(&out).trim_end_matches('\n').to_string()
}

// ---------------------------------------------------------------------------
// Ambient configuration helpers (CONFIGS.md rows 33-35)
// ---------------------------------------------------------------------------

/// Sets a locale for `category`; returns true if glibc accepted it.
pub fn try_setlocale(category: c_int, name: &str) -> bool {
    let c = format!("{name}\0");
    unsafe { !setlocale(category, c.as_ptr() as *const c_char).is_null() }
}

pub fn set_c_locale() {
    assert!(try_setlocale(LC_ALL, "C"), "cannot restore the C locale");
}

pub const FE_TONEAREST: c_int = 0;
pub const FE_DOWNWARD: c_int = 0x400;
pub const FE_UPWARD: c_int = 0x800;
pub const FE_TOWARDZERO: c_int = 0xc00;

pub fn set_round(mode: c_int) -> bool {
    unsafe { fesetround(mode) == 0 }
}

pub fn get_round() -> c_int {
    unsafe { fegetround() }
}

// ---------------------------------------------------------------------------
// Child-process execution, for error paths that poison the `FILE` (ERRORS.md
// rows 2 and 30). Running them in a fork keeps glibc's sticky stream error
// state out of the rest of the suite, and makes "did it abort?" observable:
// the crate is built with `panic = "abort"`, so a panic shows up as SIGABRT.
// ---------------------------------------------------------------------------

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn fwide(stream: *mut c_void, mode: c_int) -> c_int;
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
}

/// How to make writes to fd 1 fail.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BreakStdout {
    /// `close(1)` — fd 1 is not open at all.
    Closed,
    /// fd 1 refers to a read-only description, so `write` returns `EBADF`.
    ReadOnly,
}

/// Decoded child outcome.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChildOutcome {
    Exited(i32),
    Signalled(i32),
}

fn decode(status: c_int) -> ChildOutcome {
    let s = status as u32;
    if s & 0x7f == 0 {
        ChildOutcome::Exited(((s >> 8) & 0xff) as i32)
    } else {
        ChildOutcome::Signalled((s & 0x7f) as i32)
    }
}

/// Calls `driver(value)` in a forked child whose fd 1 has been broken in the
/// requested way, and reports how the child terminated.
pub fn outcome_with_broken_stdout(f: DriverFn, value: f64, how: BreakStdout) -> ChildOutcome {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = std::io::Write::flush(&mut std::io::stdout());
    unsafe {
        fflush(stdout_file());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            match how {
                BreakStdout::Closed => {
                    close(1);
                }
                BreakStdout::ReadOnly => {
                    // O_RDONLY
                    let ro = open(b"/dev/null\0".as_ptr() as *const c_char, 0, 0);
                    if ro >= 0 {
                        dup2(ro, 1);
                    }
                }
            }
            f(value);
            // Force the buffered data at the (broken) descriptor as well, so
            // the failure is actually reached before the child exits.
            fflush(stdout_file());
            _exit(0);
        }
        let mut status: c_int = 0;
        waitpid(pid, &mut status, 0);
        decode(status)
    }
}

/// Calls `driver(value)` in a forked child with fd 1 intact, for the
/// "does it terminate normally at all" checks.
pub fn outcome_normal(f: DriverFn, value: f64) -> ChildOutcome {
    outcome_and_output_in_child(f, value, |_| {}).0
}

/// Runs `prepare(stdout_FILE)` and then `driver(value)` in a forked child whose
/// fd 1 is an inherited scratch file, and returns both how the child terminated
/// and exactly what it wrote.
///
/// Forking is what makes the stream-state error paths testable at all: `fwide`
/// and a sticky stream error flag cannot be undone on glibc's `stdout`, so they
/// would otherwise poison the rest of the suite.
pub fn outcome_and_output_in_child<P: FnOnce(*mut c_void)>(
    f: DriverFn,
    value: f64,
    prepare: P,
) -> (ChildOutcome, Vec<u8>) {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = std::io::Write::flush(&mut std::io::stdout());
    unsafe {
        let out = stdout_file();
        fflush(out);
        let scratch = make_scratch_fd();
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            dup2(scratch, 1);
            prepare(stdout_file());
            f(value);
            fflush(stdout_file());
            _exit(0);
        }
        let mut status: c_int = 0;
        waitpid(pid, &mut status, 0);

        lseek(scratch, 0, SEEK_SET);
        let mut buf = Vec::new();
        let mut chunk = [0u8; 65536];
        loop {
            let n = read(scratch, chunk.as_mut_ptr() as *mut c_void, chunk.len());
            if n <= 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n as usize]);
        }
        close(scratch);
        (decode(status), buf)
    }
}

/// `fwide` — forces or queries the byte/wide orientation of a stream.
pub fn fwide_stream(stream: *mut c_void, mode: c_int) -> c_int {
    unsafe { fwide(stream, mode) }
}

/// `setvbuf` buffering modes.
pub const IOFBF: c_int = 0;
pub const IOLBF: c_int = 1;
pub const IONBF: c_int = 2;

/// `setvbuf` with a library-allocated buffer.
pub fn setvbuf_stream(stream: *mut c_void, mode: c_int) -> c_int {
    unsafe { setvbuf(stream, std::ptr::null_mut(), mode, 0) }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_D1FF_2025;

pub struct Rng(u64);

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
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        debug_assert!(lo <= hi);
        lo + self.below(hi - lo + 1)
    }
    /// A `f64` uniform in `[0, 1)` (53-bit).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
}

// ---------------------------------------------------------------------------
// Bit-pattern construction helpers
// ---------------------------------------------------------------------------

pub const MANT_MASK: u64 = 0x000f_ffff_ffff_ffff;
pub const SIGN: u64 = 0x8000_0000_0000_0000;

/// Assembles a binary64 bit pattern from its three fields.
pub fn bits_of(sign: bool, expfield: u64, mantissa: u64) -> u64 {
    debug_assert!(expfield <= 0x7ff);
    (if sign { SIGN } else { 0 }) | (expfield << 52) | (mantissa & MANT_MASK)
}

/// Uniformly random value whose magnitude lies in `[lo, hi)`, both signs.
pub fn random_in_range(rng: &mut Rng, lo: f64, hi: f64, n: usize) -> Vec<u64> {
    let mut v = Vec::with_capacity(n);
    while v.len() < n {
        let x = lo + rng.unit() * (hi - lo);
        if !(x.is_finite()) {
            continue;
        }
        let neg = rng.next_u64() & 1 == 1;
        v.push(if neg { (-x).to_bits() } else { x.to_bits() });
    }
    v
}
