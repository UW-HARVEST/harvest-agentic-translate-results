//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! driven only through their exported `extern "C"` symbols — the Rust functions
//! are never called directly, so the `#[no_mangle]` wrappers are under test too.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Build configuration mirroring (must match src/mdmacros.rs exactly)
// ---------------------------------------------------------------------------

/// The `OP` value this test binary was compiled for.
pub const OP: &str = if cfg!(feature = "mul") {
    "mul"
} else if cfg!(feature = "sub") {
    "sub"
} else {
    "add"
};

/// The `REPEAT` value this test binary was compiled for.
pub const REPEAT: c_int = if cfg!(feature = "7") {
    7
} else if cfg!(feature = "6") {
    6
} else if cfg!(feature = "5") {
    5
} else if cfg!(feature = "4") {
    4
} else if cfg!(feature = "3") {
    3
} else if cfg!(feature = "2") {
    2
} else if cfg!(feature = "1") {
    1
} else if cfg!(feature = "0") {
    0
} else {
    5
};

/// The C `INIT_FOR(OP)` value.
pub const INIT: c_int = if cfg!(feature = "mul") { 1 } else { 0 };

/// The C `STEP_OP(OP, acc, i)`, used only to sanity-check expectations.
pub fn step(acc: c_int, i: c_int) -> c_int {
    match OP {
        "mul" => acc.wrapping_mul(i.wrapping_add(1)),
        "sub" => acc.wrapping_sub(i),
        _ => acc.wrapping_add(i),
    }
}

// ---------------------------------------------------------------------------
// Artifact locations
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    manifest_dir().parent().expect("no parent dir").to_path_buf()
}

/// Path to the C `.so` built for this exact `OP`/`REPEAT` configuration.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    workspace_root()
        .join("cbuild")
        .join(format!("libcdriver_{}_{}.so", OP, REPEAT))
}

/// Path to the C `driver` executable built for this configuration.
pub fn c_bin_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_BIN") {
        return PathBuf::from(p);
    }
    workspace_root()
        .join("cbuild")
        .join(format!("driver_{}_{}", OP, REPEAT))
}

/// Path to the Rust `cdylib`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // The integration-test binary lives in target/<profile>/deps/, so the
    // cdylib is one directory up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf();
    profile_dir.join("libdriver.so")
}

/// Path to the Rust `driver` executable.
pub fn rust_bin_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_BIN") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf();
    profile_dir.join("driver")
}

// ---------------------------------------------------------------------------
// Loaded library wrapper
// ---------------------------------------------------------------------------

pub type Bin = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type Un = unsafe extern "C" fn(c_int) -> c_int;

pub struct Loaded {
    pub lib: Library,
    pub path: PathBuf,
}

impl Loaded {
    fn open(path: PathBuf) -> Loaded {
        assert!(
            path.exists(),
            "shared object not found: {}\n\
             (build the C libs with ../build_c.sh and the Rust cdylib with `cargo build`)",
            path.display()
        );
        // SAFETY: loading a shared object we just built ourselves.
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        Loaded { lib, path }
    }

    pub fn bin(&self, name: &str) -> Symbol<'_, Bin> {
        // SAFETY: the C signature is `int name(int, int)`.
        unsafe { self.lib.get(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("{}: missing symbol {name}: {e}", self.path.display()))
    }

    pub fn un(&self, name: &str) -> Symbol<'_, Un> {
        // SAFETY: the C signature is `int name(int)`.
        unsafe { self.lib.get(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("{}: missing symbol {name}: {e}", self.path.display()))
    }

    /// Address of an exported *function* symbol (for identity checks).
    pub fn fn_addr(&self, name: &str) -> usize {
        *self.bin(name) as usize
    }

    /// `int (*G_OP)(int,int)` — reads the global function pointer and returns it
    /// as a callable.
    pub fn g_op(&self) -> Bin {
        // SAFETY: `G_OP` is an exported object of type `int (*)(int,int)`.
        let sym: Symbol<'_, *const Bin> = unsafe { self.lib.get(b"G_OP") }
            .unwrap_or_else(|e| panic!("{}: missing symbol G_OP: {e}", self.path.display()));
        unsafe { **sym }
    }

    /// The raw pointer *value* stored in `G_OP` (used for identity checks).
    pub fn g_op_value(&self) -> usize {
        self.g_op() as usize
    }

    /// Overwrites the *writable* global `G_OP` with another function pointer.
    ///
    /// # Safety
    /// Mutates library-global state; callers must hold [`capture_lock`] or
    /// otherwise ensure no concurrent use.
    pub unsafe fn set_g_op(&self, f: Bin) {
        let sym: Symbol<'_, *mut Bin> = unsafe { self.lib.get(b"G_OP") }
            .unwrap_or_else(|e| panic!("{}: missing symbol G_OP: {e}", self.path.display()));
        unsafe { **sym = f };
    }

    /// `const char *G_OP_NAME` — the pointed-to NUL-terminated bytes.
    pub fn g_op_name(&self) -> Vec<u8> {
        // SAFETY: `G_OP_NAME` is an exported object of type `const char *`
        // pointing at a static NUL-terminated string literal.
        let sym: Symbol<'_, *const *const c_char> = unsafe { self.lib.get(b"G_OP_NAME") }
            .unwrap_or_else(|e| panic!("{}: missing symbol G_OP_NAME: {e}", self.path.display()));
        let p = unsafe { **sym };
        assert!(!p.is_null(), "{}: G_OP_NAME is NULL", self.path.display());
        unsafe { CStr::from_ptr(p) }.to_bytes().to_vec()
    }
}

fn c_lib() -> &'static Loaded {
    static L: OnceLock<Loaded> = OnceLock::new();
    L.get_or_init(|| Loaded::open(c_so_path()))
}

fn rust_lib() -> &'static Loaded {
    static L: OnceLock<Loaded> = OnceLock::new();
    L.get_or_init(|| Loaded::open(rust_so_path()))
}

/// The two implementations under comparison.
pub fn pair() -> (&'static Loaded, &'static Loaded) {
    (c_lib(), rust_lib())
}

// ---------------------------------------------------------------------------
// stdout capture (the C functions `printf`, the Rust ones `println!`)
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open C stream in this process — including
    /// the `stdout` `FILE*` that the loaded C `.so` writes through.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Serializes fd-1 redirection across the (multi-threaded) test harness.
pub fn capture_lock() -> MutexGuard<'static, ()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` with file descriptor 1 redirected to a temporary file and returns
/// `(f's return value, captured stdout bytes)`.
pub fn capture_stdout<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    use std::io::{Read, Seek, Write};
    use std::os::unix::io::AsRawFd;

    let _guard = capture_lock();

    // Make sure nothing already-buffered lands in our capture file.
    let _ = std::io::stdout().flush();
    // SAFETY: flushing all C streams has no preconditions.
    unsafe { fflush(std::ptr::null_mut()) };

    let mut tmp = tempfile();

    // SAFETY: plain POSIX fd juggling on fds we own.
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(tmp.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let out = f();

    let _ = std::io::stdout().flush();
    // SAFETY: as above.
    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };

    let mut buf = Vec::new();
    tmp.rewind().expect("rewind");
    tmp.read_to_end(&mut buf).expect("read capture");
    (out, buf)
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    let path = PathBuf::from(dir).join(format!(
        "driver_capture_{}_{}_{}.txt",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed),
        OP
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create capture file");
    // Unlink immediately; the fd keeps it alive.
    let _ = std::fs::remove_file(&path);
    f
}

// ---------------------------------------------------------------------------
// Differential helpers
// ---------------------------------------------------------------------------

/// Calls `name(a, b)` in both `.so`s and asserts identical return value *and*
/// identical stdout bytes.
pub fn diff_bin(name: &str, a: c_int, b: c_int) {
    let (c, r) = pair();
    let cf = c.bin(name);
    let rf = r.bin(name);
    // SAFETY: signature matches the C declaration `int name(int,int)`.
    let (cv, cout) = capture_stdout(|| unsafe { cf(a, b) });
    let (rv, rout) = capture_stdout(|| unsafe { rf(a, b) });
    assert_eq!(
        cv, rv,
        "{name}({a}, {b}) return mismatch [OP={OP} REPEAT={REPEAT}]"
    );
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "{name}({a}, {b}) stdout mismatch [OP={OP} REPEAT={REPEAT}]"
    );
}

/// Calls `name(n)` in both `.so`s and asserts identical return value and stdout.
pub fn diff_un(name: &str, n: c_int) {
    let (c, r) = pair();
    let cf = c.un(name);
    let rf = r.un(name);
    // SAFETY: signature matches the C declaration `int name(int)`.
    let (cv, cout) = capture_stdout(|| unsafe { cf(n) });
    let (rv, rout) = capture_stdout(|| unsafe { rf(n) });
    assert_eq!(cv, rv, "{name}({n}) return mismatch [OP={OP} REPEAT={REPEAT}]");
    assert_eq!(
        String::from_utf8_lossy(&cout),
        String::from_utf8_lossy(&rout),
        "{name}({n}) stdout mismatch [OP={OP} REPEAT={REPEAT}]"
    );
}

/// Calls `G_OP(a, b)` in both `.so`s and compares.
pub fn diff_g_op(a: c_int, b: c_int) {
    let (c, r) = pair();
    let cf = c.g_op();
    let rf = r.g_op();
    // SAFETY: `G_OP` holds `int (*)(int,int)`.
    let cv = unsafe { cf(a, b) };
    let rv = unsafe { rf(a, b) };
    assert_eq!(cv, rv, "G_OP({a}, {b}) mismatch [OP={OP} REPEAT={REPEAT}]");
}

// ---------------------------------------------------------------------------
// Deterministic PRNG + input corpora
// ---------------------------------------------------------------------------

/// SplitMix64 — fixed seed, so every run uses the same inputs.
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
    /// A full-range `int`, with extra weight on small magnitudes and on the
    /// signed extremes (so overflow paths are hit often).
    pub fn next_int(&mut self) -> c_int {
        let w = self.next_u64();
        let small = ((w >> 8) % 5) as i32; // 0..=4
        let tiny = ((w >> 8) % 3) as i32; // 0..=2
        match w % 8 {
            0 => small,
            1 => -small,
            2 => i32::MAX - tiny,
            3 => i32::MIN + tiny,
            _ => (w >> 32) as u32 as i32,
        }
    }
    /// An `int` biased towards the `DISPATCH_REP` window `-2..=9`.
    pub fn next_n(&mut self) -> c_int {
        let w = self.next_u64();
        if w % 2 == 0 {
            (((w >> 8) % 12) as i32) - 2
        } else {
            self.next_int()
        }
    }
}

pub const SEED: u64 = 0x5DEE_CE66_D;

/// Signed boundary values.
pub const BOUNDS: [c_int; 7] = [
    0,
    1,
    -1,
    c_int::MIN,
    c_int::MIN + 1,
    c_int::MAX,
    c_int::MAX - 1,
];

/// Exhaustive small grid `[-4..=4]`.
pub fn small_grid() -> impl Iterator<Item = (c_int, c_int)> {
    (-4..=4).flat_map(|a| (-4..=4).map(move |b| (a, b)))
}

/// Cross product of the boundary values.
pub fn bounds_grid() -> impl Iterator<Item = (c_int, c_int)> {
    BOUNDS
        .into_iter()
        .flat_map(|a| BOUNDS.into_iter().map(move |b| (a, b)))
}

/// Number of randomized iterations per property-style row.
pub const ITERS: usize = 512;
