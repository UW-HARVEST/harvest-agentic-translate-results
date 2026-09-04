// Differential-test harness.
//
// BOTH implementations are loaded as shared objects through `libloading` and
// called only through their exported C symbols — the Rust functions are never
// called directly, so the `#[no_mangle] extern "C"` wrappers are part of what is
// under test.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need for env manipulation, fd redirection and fork/wait.
// ---------------------------------------------------------------------------

extern "C" {
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    fn unsetenv(name: *const c_char) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

// ---------------------------------------------------------------------------
// The `struct ConfigFlags` allocation unit: one 4-byte `unsigned int`, of which
// only bits 0..7 are declared as bit-fields.
// ---------------------------------------------------------------------------

#[repr(C, align(4))]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Flags(pub [u8; 4]);

pub const F_VERBOSE: u8 = 1 << 0;
pub const F_DEBUG: u8 = 1 << 1;
pub const F_OPTIMIZE: u8 = 1 << 2;
pub const F_CACHE: u8 = 1 << 3;
pub const F_RESERVED: u8 = 1 << 7;

impl Flags {
    /// Build the allocation unit from the individual bit-fields.
    pub fn new(verbose: bool, debug: bool, optimize: bool, cache: bool, log_level: u8) -> Flags {
        let mut b = 0u8;
        if verbose {
            b |= F_VERBOSE;
        }
        if debug {
            b |= F_DEBUG;
        }
        if optimize {
            b |= F_OPTIMIZE;
        }
        if cache {
            b |= F_CACHE;
        }
        b |= (log_level & 0x7) << 4;
        Flags([b, 0, 0, 0])
    }
    /// Raw byte-0 pattern plus arbitrary garbage in bits 8..31.
    pub fn raw(byte0: u8, upper: [u8; 3]) -> Flags {
        Flags([byte0, upper[0], upper[1], upper[2]])
    }
    pub fn as_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }
}

impl std::fmt::Debug for Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Flags(0x{:02x}{:02x}{:02x}{:02x} [v={} d={} o={} c={} log={} r={}])",
            self.0[3],
            self.0[2],
            self.0[1],
            self.0[0],
            self.0[0] & 1,
            (self.0[0] >> 1) & 1,
            (self.0[0] >> 2) & 1,
            (self.0[0] >> 3) & 1,
            (self.0[0] >> 4) & 7,
            (self.0[0] >> 7) & 1
        )
    }
}

// ---------------------------------------------------------------------------
// Library location + loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/lib<project>.so` — the project name is derived by CMake from the
/// parent directory name, so the file is located by extension instead.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().parent().unwrap().join("c_src/build");
    let mut found: Vec<PathBuf> = fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} (build the C library first)", build.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    assert_eq!(found.len(), 1, "expected exactly one .so in {}, got {found:?}", build.display());
    found.pop().unwrap()
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libenvy_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!("libenvy_lib.so not found under {}; run `cargo build --release`", base.display());
}

pub type FnParseEnvNumeric = unsafe extern "C" fn(*const c_char, c_int) -> c_int;
pub type FnInitConfig = unsafe extern "C" fn(*mut u8);
pub type FnPerformOperation = unsafe extern "C" fn(c_int, c_int, *mut u8) -> c_int;
pub type FnApplyBitOps = unsafe extern "C" fn(c_int, *mut u8) -> c_int;
pub type FnEnvy = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// One loaded implementation, reached exclusively through its dynamic symbols.
pub struct Impl {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub parse_env_numeric: FnParseEnvNumeric,
    pub init_config_from_env: FnInitConfig,
    pub perform_operation: FnPerformOperation,
    pub apply_bit_operations: FnApplyBitOps,
    pub envy: FnEnvy,
}

impl Impl {
    unsafe fn load(name: &'static str, path: &Path) -> Impl {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
        macro_rules! sym {
            ($t:ty, $n:literal) => {{
                let s: Symbol<$t> = lib
                    .get($n)
                    .unwrap_or_else(|e| panic!("{} misses symbol {}: {e}", path.display(), stringify!($n)));
                *s
            }};
        }
        let parse_env_numeric = sym!(FnParseEnvNumeric, b"parse_env_numeric\0");
        let init_config_from_env = sym!(FnInitConfig, b"init_config_from_env\0");
        let perform_operation = sym!(FnPerformOperation, b"perform_operation\0");
        let apply_bit_operations = sym!(FnApplyBitOps, b"apply_bit_operations\0");
        let envy = sym!(FnEnvy, b"envy\0");
        Impl {
            name,
            path: path.to_path_buf(),
            _lib: lib,
            parse_env_numeric,
            init_config_from_env,
            perform_operation,
            apply_bit_operations,
            envy,
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rs: Impl,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

/// Loads both `.so`s (once per test process) and takes the global lock that
/// serialises the two process-wide resources these tests manipulate: the
/// environment and the stdout/stderr file descriptors.
pub fn pair() -> (&'static Pair, MutexGuard<'static, ()>) {
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let p = PAIR.get_or_init(|| unsafe {
        Pair {
            c: Impl::load("C", &c_so_path()),
            rs: Impl::load("Rust", &rust_so_path()),
        }
    });
    (p, guard)
}

// ---------------------------------------------------------------------------
// Environment helpers (operate on the real `environ`, which is what the
// libraries' `getenv` calls consult).
// ---------------------------------------------------------------------------

pub fn set_env(name: &str, value: &str) {
    let n = CString::new(name).unwrap();
    let v = CString::new(value).unwrap();
    unsafe {
        assert_eq!(setenv(n.as_ptr(), v.as_ptr(), 1), 0, "setenv({name}) failed");
    }
}

pub fn unset_env(name: &str) {
    let n = CString::new(name).unwrap();
    unsafe {
        unsetenv(n.as_ptr());
    }
}

/// `None` ⇒ unset, `Some(v)` ⇒ set to `v`.
pub fn apply_env(name: &str, value: Option<&str>) {
    match value {
        None => unset_env(name),
        Some(v) => set_env(name, v),
    }
}

pub const ENV_NAMES: [&str; 5] = [
    "PROG_VERBOSE",
    "PROG_DEBUG",
    "PROG_OPTIMIZE",
    "PROG_BASE_OFFSET",
    "PROG_MULTIPLIER",
];

pub fn clear_prog_env() {
    for n in ENV_NAMES {
        unset_env(n);
    }
}

// ---------------------------------------------------------------------------
// stdout / stderr capture
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Clone)]
pub struct Output {
    pub out: Vec<u8>,
    pub err: Vec<u8>,
}

impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ stdout: {:?}, stderr: {:?} }}",
            String::from_utf8_lossy(&self.out),
            String::from_utf8_lossy(&self.err)
        )
    }
}

fn tmp_path(tag: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("difftest_{}_{}_{}.txt", std::process::id(), tag, n))
}

/// Runs `f` with fds 1 and 2 redirected to temporary files and returns both the
/// value `f` produced and the exact bytes it wrote.
pub fn capture<R, F: FnOnce() -> R>(f: F) -> (R, Output) {
    let po = tmp_path("out");
    let pe = tmp_path("err");
    let fo = fs::File::create(&po).unwrap();
    let fe = fs::File::create(&pe).unwrap();
    let r;
    let out;
    let err;
    // Drain Rust's own buffered stdout/stderr (libtest keeps a partial
    // "test <name> ... " line pending) so that it cannot be flushed into the
    // redirected fds and be mistaken for library output.
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
    }
    unsafe {
        fflush(std::ptr::null_mut());
        let saved_out = dup(1);
        let saved_err = dup(2);
        assert!(saved_out >= 0 && saved_err >= 0);
        dup2(fo.as_raw_fd(), 1);
        dup2(fe.as_raw_fd(), 2);
        r = f();
        fflush(std::ptr::null_mut());
        dup2(saved_out, 1);
        dup2(saved_err, 2);
        close(saved_out);
        close(saved_err);
    }
    drop(fo);
    drop(fe);
    out = fs::read(&po).unwrap();
    err = fs::read(&pe).unwrap();
    let _ = fs::remove_file(&po);
    let _ = fs::remove_file(&pe);
    (r, Output { out, err })
}

/// Result of one differential call: return value + byte-exact output.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Call {
    pub ret: c_int,
    pub output: Output,
}

pub fn call<F: FnOnce() -> c_int>(f: F) -> Call {
    let (ret, output) = capture(f);
    Call { ret, output }
}

/// Compares two differential results and panics with full context on mismatch.
#[track_caller]
pub fn assert_same(ctx: &str, c: &Call, rs: &Call) {
    if c.ret != rs.ret || c.output != rs.output {
        panic!(
            "DIVERGENCE [{ctx}]\n  C   : ret={:?} {:?}\n  Rust: ret={:?} {:?}",
            c.ret, c.output, rs.ret, rs.output
        );
    }
}

#[track_caller]
pub fn assert_same_flags(ctx: &str, c: (&Flags, &Output), rs: (&Flags, &Output)) {
    if c.0 != rs.0 || c.1 != rs.1 {
        panic!(
            "DIVERGENCE [{ctx}]\n  C   : {:?} {:?}\n  Rust: {:?} {:?}",
            c.0, c.1, rs.0, rs.1
        );
    }
}

// ---------------------------------------------------------------------------
// Crash comparison: run `f` in a forked child and report its raw wait status,
// so that "both implementations die with the same signal" can be asserted.
// ---------------------------------------------------------------------------

pub fn child_status<F: FnOnce()>(f: F) -> i32 {
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Silence the child's output so a crash message cannot pollute the
            // parent's test log, then run the offending call.
            let devnull = fs::File::create("/dev/null").unwrap();
            dup2(devnull.as_raw_fd(), 1);
            dup2(devnull.as_raw_fd(), 2);
            f();
            _exit(0);
        }
        let mut status: c_int = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        status
    }
}

/// Asserts both implementations died the *same* way, and that they really did
/// die (so the row cannot pass vacuously by both returning normally).
#[track_caller]
pub fn assert_same_fatal(ctx: &str, sc: i32, sr: i32) {
    assert_eq!(
        status_desc(sc),
        status_desc(sr),
        "{ctx}: C {} vs Rust {}",
        status_desc(sc),
        status_desc(sr)
    );
    assert_ne!(sc & 0x7f, 0, "{ctx}: expected a fatal signal, got {}", status_desc(sc));
    assert_eq!(sc & 0x7f, 11, "{ctx}: expected SIGSEGV, got {}", status_desc(sc));
}

/// Human-readable classification of a raw wait status (exit code vs signal).
pub fn status_desc(status: i32) -> String {
    if status & 0x7f == 0 {
        format!("exited({})", (status >> 8) & 0xff)
    } else {
        format!("signal({})", status & 0x7f)
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed, reproducible runs.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234;

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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    /// Mixes uniform 32-bit values with small values and hard boundaries, so
    /// both "random" and "interesting" inputs are covered.
    pub fn interesting_i32(&mut self) -> i32 {
        match self.below(10) {
            0 => i32::MIN,
            1 => i32::MAX,
            2 => 0,
            3 => -1,
            4 => 1,
            5 => 0x4000_0000u32 as i32,
            6 => 0x3FFF_FFFF,
            7 => -(self.below(1000) as i32),
            8 => self.below(1000) as i32,
            _ => self.next_i32(),
        }
    }
}

// ---------------------------------------------------------------------------
// Independent reference model of `envy`, written straight off `c_src/src/lib.c`.
// It exists so tests can assert *absolute* expected values (e.g. "the restore
// branch returns param1") without hand-computing arithmetic that the final
// `| 0x0F` and `+ base_offset` steps make easy to get wrong.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct Cfg {
    pub verbose: bool,
    pub debug: bool,
    pub optimize: bool,
    pub base_offset: i32,
    pub multiplier: i32,
}

impl Default for Cfg {
    /// The configuration produced by an environment with none of the five
    /// variables set: `log_level = 03`, `cache_enabled = 1`, `0100`, `012`.
    fn default() -> Cfg {
        Cfg { verbose: false, debug: false, optimize: false, base_offset: 0o100, multiplier: 0o12 }
    }
}

/// Returns `(return value, whether the `result < 0` restore branch was taken)`.
pub fn model_envy(cfg: &Cfg, p1: i32, p2: i32, p3: i32, p4: i32) -> (i32, bool) {
    const LOG_LEVEL: i32 = 0o3; // init_config_from_env hard-wires this
    let mut r = if cfg.optimize {
        p1.wrapping_add(p2)
    } else {
        p1.wrapping_mul(LOG_LEVEL).wrapping_add(p2.wrapping_div(2))
    };
    if p3 != 0 {
        r = r.wrapping_add(p3.wrapping_mul(cfg.multiplier));
    }
    if p4 != 0 {
        r = r.wrapping_add(p4 >> 2);
    }
    if cfg.verbose {
        r = ((r as u32) << 1) as i32; // cache_enabled is always 1 below
    }
    r |= 0x0F;
    r = r.wrapping_add(cfg.base_offset);
    if r < 0 {
        (p1, true) // state restored from backup ⇒ base_value == param1
    } else {
        (r, false)
    }
}

/// The `int` boundary values every parameter is additionally swept over.
pub const BOUNDARIES: [i32; 7] = [
    i32::MIN,
    -1,
    0,
    1,
    i32::MAX,
    0x3FFF_FFFF,
    0x4000_0000u32 as i32,
];
