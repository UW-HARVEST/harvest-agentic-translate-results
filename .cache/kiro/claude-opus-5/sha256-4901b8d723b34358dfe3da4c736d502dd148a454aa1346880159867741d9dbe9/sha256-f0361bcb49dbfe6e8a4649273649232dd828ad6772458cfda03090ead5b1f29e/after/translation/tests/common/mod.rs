//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects through `libloading` and every
//! call goes through their exported `extern "C"` symbols — the Rust crate is
//! never linked or called directly, so the `#[no_mangle]` wrappers are part of
//! what is under test.
#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CString};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits the harness needs (env manipulation, fd redirection, fork).
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    fn unsetenv(name: *const c_char) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

// ---------------------------------------------------------------------------
// The exported API surface, as raw C function pointers pulled out of a `.so`.
// ---------------------------------------------------------------------------

/// `struct ConfigFlags` is a single 4-byte `unsigned int` storage unit holding
/// six bit-fields in its low byte. It is passed across FFI as a `*mut u32`.
pub type FlagsWord = u32;

pub const BIT_VERBOSE: u32 = 1 << 0;
pub const BIT_DEBUG: u32 = 1 << 1;
pub const BIT_OPTIMIZE: u32 = 1 << 2;
pub const BIT_CACHE: u32 = 1 << 3;
pub const LOG_LEVEL_SHIFT: u32 = 4;
pub const BIT_RESERVED: u32 = 1 << 7;

/// Build a flag word from the individual bit-field values.
pub fn flag_word(verbose: u32, debug: u32, optimize: u32, cache: u32, log_level: u32, reserved: u32) -> FlagsWord {
    (verbose & 1)
        | ((debug & 1) << 1)
        | ((optimize & 1) << 2)
        | ((cache & 1) << 3)
        | ((log_level & 7) << LOG_LEVEL_SHIFT)
        | ((reserved & 1) << 7)
}

#[derive(Clone, Copy)]
pub struct Api {
    pub name: &'static str,
    pub parse_env_numeric: unsafe extern "C" fn(*const c_char, c_int) -> c_int,
    pub init_config_from_env: unsafe extern "C" fn(*mut FlagsWord),
    pub perform_operation: unsafe extern "C" fn(c_int, c_int, *mut FlagsWord) -> c_int,
    pub apply_bit_operations: unsafe extern "C" fn(c_int, *mut FlagsWord) -> c_int,
    pub envy: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFFTEST_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DIFFTEST_C_SO={} is not a file", p.display());
        return p;
    }
    let dir = workspace_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "so").unwrap_or(false) {
                found.push(p);
            }
        }
    }
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {:?}. Build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        dir.display(),
        found
    );
    found.pop().unwrap()
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DIFFTEST_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DIFFTEST_RUST_SO={} is not a file", p.display());
        return p;
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    // Prefer the profile the tests were built with, then fall back.
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libenvy_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "libenvy_lib.so not found under {}. Build it first: cd translation && cargo build --release",
        base.display()
    );
}

fn load(path: &PathBuf, name: &'static str) -> Api {
    // Leaked on purpose: the function pointers must stay valid for the whole
    // process lifetime, and dlclose-ing mid-test would invalidate them.
    let lib: &'static Library = Box::leak(Box::new(
        unsafe { Library::new(path) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display())),
    ));
    macro_rules! sym {
        ($n:literal, $t:ty) => {{
            let s = unsafe { lib.get::<$t>(concat!($n, "\0").as_bytes()) }
                .unwrap_or_else(|e| panic!("{} missing symbol {}: {e}", name, $n));
            *s
        }};
    }
    Api {
        name,
        parse_env_numeric: sym!(
            "parse_env_numeric",
            unsafe extern "C" fn(*const c_char, c_int) -> c_int
        ),
        init_config_from_env: sym!("init_config_from_env", unsafe extern "C" fn(*mut FlagsWord)),
        perform_operation: sym!(
            "perform_operation",
            unsafe extern "C" fn(c_int, c_int, *mut FlagsWord) -> c_int
        ),
        apply_bit_operations: sym!(
            "apply_bit_operations",
            unsafe extern "C" fn(c_int, *mut FlagsWord) -> c_int
        ),
        envy: sym!(
            "envy",
            unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int
        ),
    }
}

pub struct Both {
    pub c: Api,
    pub rs: Api,
}

static BOTH: OnceLock<Both> = OnceLock::new();

pub fn both() -> &'static Both {
    BOTH.get_or_init(|| Both {
        c: load(&c_so_path(), "C"),
        rs: load(&rust_so_path(), "Rust"),
    })
}

// ---------------------------------------------------------------------------
// Environment is process-global state shared by both libraries, so every test
// that touches it must hold this lock.
// ---------------------------------------------------------------------------
static ENV_LOCK: Mutex<()> = Mutex::new(());

pub fn env_guard() -> MutexGuard<'static, ()> {
    match ENV_LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

pub const PROG_VARS: [&str; 5] = [
    "PROG_VERBOSE",
    "PROG_DEBUG",
    "PROG_OPTIMIZE",
    "PROG_BASE_OFFSET",
    "PROG_MULTIPLIER",
];

pub fn clear_prog_env() {
    for v in PROG_VARS {
        let c = CString::new(v).unwrap();
        unsafe { unsetenv(c.as_ptr()) };
    }
}

pub fn set_env(name: &str, value: &str) {
    let n = CString::new(name).unwrap();
    let v = CString::new(value).unwrap();
    assert_eq!(unsafe { setenv(n.as_ptr(), v.as_ptr(), 1) }, 0);
}

pub fn unset_env(name: &str) {
    let n = CString::new(name).unwrap();
    unsafe { unsetenv(n.as_ptr()) };
}

/// Apply an environment configuration: `None` means "unset".
pub fn apply_env(cfg: &[(&str, Option<&str>)]) {
    clear_prog_env();
    for (k, v) in cfg {
        match v {
            Some(val) => set_env(k, val),
            None => unset_env(k),
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seed for reproducibility.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_5678_9ABC;

impl Rng {
    pub fn new() -> Self {
        Rng(SEED)
    }
    pub fn with_seed(s: u64) -> Self {
        Rng(if s == 0 { 1 } else { s })
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
    /// Values biased toward small magnitudes as well as full-range extremes, so
    /// both ordinary arithmetic and overflow paths get hit.
    pub fn next_i32_mixed(&mut self) -> i32 {
        let r = self.next_u64();
        match r & 0x7 {
            0 | 1 => (r >> 32) as i32,                       // full range
            2 => ((r >> 32) as i32) % 256,                   // tiny
            3 => ((r >> 32) as i32) % 65536,                 // small
            4 => i32::MAX - ((r >> 32) as u32 % 8) as i32,    // near INT_MAX
            5 => i32::MIN.wrapping_add(((r >> 32) as u32 % 8) as i32), // near INT_MIN
            6 => ((r >> 32) as i32) / 2,                     // half range
            _ => ((r >> 32) as i32) % 1000 - 500,            // around zero
        }
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

/// The boundary vector used everywhere a "one step past the range" sweep is
/// called for.
pub const EXTREMES: [i32; 9] = [
    i32::MIN,
    i32::MIN + 1,
    -2,
    -1,
    0,
    1,
    2,
    i32::MAX - 1,
    i32::MAX,
];

// ---------------------------------------------------------------------------
// stdout / stderr capture. Both libraries print through the process-wide libc
// FILE streams, so capture works by redirecting fds 1 and 2 to temp files.
// ---------------------------------------------------------------------------
pub struct Captured {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

static CAP_SEQ: Mutex<u64> = Mutex::new(0);

fn temp_path(tag: &str) -> PathBuf {
    let mut seq = CAP_SEQ.lock().unwrap();
    *seq += 1;
    std::env::temp_dir().join(format!("difftest-{}-{}-{}.out", std::process::id(), tag, *seq))
}

/// Run `f` with fds 1 and 2 redirected, and return everything it wrote.
pub fn capture<R, F: FnOnce() -> R>(f: F) -> (R, Captured) {
    let out_path = temp_path("out");
    let err_path = temp_path("err");
    let out_file = std::fs::File::create(&out_path).unwrap();
    let err_file = std::fs::File::create(&err_path).unwrap();

    unsafe {
        fflush(std::ptr::null_mut());
        let saved_out = dup(1);
        let saved_err = dup(2);
        assert!(saved_out >= 0 && saved_err >= 0, "dup failed");
        assert!(dup2(out_file.as_raw_fd(), 1) >= 0, "dup2 stdout failed");
        assert!(dup2(err_file.as_raw_fd(), 2) >= 0, "dup2 stderr failed");

        let r = f();

        fflush(std::ptr::null_mut());
        dup2(saved_out, 1);
        dup2(saved_err, 2);
        close(saved_out);
        close(saved_err);

        drop(out_file);
        drop(err_file);

        let mut so = Vec::new();
        std::fs::File::open(&out_path).unwrap().read_to_end(&mut so).unwrap();
        let mut se = Vec::new();
        std::fs::File::open(&err_path).unwrap().read_to_end(&mut se).unwrap();
        let _ = std::fs::remove_file(&out_path);
        let _ = std::fs::remove_file(&err_path);

        (r, Captured { stdout: so, stderr: se })
    }
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

// ---------------------------------------------------------------------------
// Fatal-signal comparison: run a closure in a forked child and report how it
// died. Used for the null-pointer rows, where the C is UB/fatal and the Rust
// must be *equally* fatal (same signal), not merely "also broken".
// ---------------------------------------------------------------------------
#[derive(Debug, PartialEq, Eq)]
pub enum Death {
    Exited(i32),
    Signaled(i32),
}

pub fn run_in_child<F: FnOnce()>(f: F) -> Death {
    unsafe {
        fflush(std::ptr::null_mut());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: only the library call, then a hard exit. No allocation,
            // no unwinding, no atexit handlers.
            f();
            _exit(0);
        }
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        let s = status as u32;
        if s & 0x7f == 0x7f {
            // stopped; shouldn't happen
            Death::Signaled(-1)
        } else if s & 0x7f != 0 {
            Death::Signaled((s & 0x7f) as i32)
        } else {
            Death::Exited(((s >> 8) & 0xff) as i32)
        }
    }
}

/// Assert both libraries agree on an `i32` result, with a descriptive message.
#[track_caller]
pub fn eq_i32(ctx: &str, c: i32, rs: i32) {
    assert_eq!(
        c, rs,
        "DIVERGENCE [{ctx}]: C returned {c} (0x{c:08x}), Rust returned {rs} (0x{rs:08x})"
    );
}
