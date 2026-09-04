//! Shared differential-test harness.
//!
//! Both libraries are loaded as shared objects through `libloading` and driven
//! only through their exported symbols; the Rust implementation is never called
//! directly, so the `#[unsafe(no_mangle)] extern "C"` wrappers are under test
//! too.
//!
//! * C   `.so`: `cbuild/libcmd_<op>_<repeat>.so` (built by `cbuild/build_c.sh`)
//! * Rust `.so`: `translation/target/<profile>/libdriver.so`
//!
//! Which C `.so` to load is decided from the Cargo features of *this* test
//! target (features are per-package, so the tests are compiled with the same
//! feature set as the library). The mapping used here is the direct one
//! (`repeat_N` -> `N`, `add|sub|mul` -> that op); it deliberately does **not**
//! reuse `src/mdmacros.rs`'s cfg-priority chain, so a bug in that chain shows up
//! as a C/Rust divergence instead of cancelling out.

#![allow(dead_code)]

use std::ffi::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// `int (*)(int, int)`
pub type BinOp = unsafe extern "C" fn(c_int, c_int) -> c_int;
/// `int (*)(int)`
pub type UnOp = unsafe extern "C" fn(c_int) -> c_int;

/* ------------------------------------------------------------------ *
 * Build configuration of this test binary.
 * ------------------------------------------------------------------ */

/// The `OP` cache variable this test binary was compiled for.
pub const OP: &str = if cfg!(feature = "sub") {
    "sub"
} else if cfg!(feature = "mul") {
    "mul"
} else {
    // `add`, and the `#ifndef OP / #define OP add` fallback.
    "add"
};

/// The `REPEAT` cache variable this test binary was compiled for.
pub const REPEAT: c_int = if cfg!(feature = "repeat_0") {
    0
} else if cfg!(feature = "repeat_1") {
    1
} else if cfg!(feature = "repeat_2") {
    2
} else if cfg!(feature = "repeat_3") {
    3
} else if cfg!(feature = "repeat_4") {
    4
} else if cfg!(feature = "repeat_6") {
    6
} else if cfg!(feature = "repeat_7") {
    7
} else {
    // `repeat_5`, and the `#ifndef REPEAT / #define REPEAT 5` fallback.
    5
};

/// `INIT_<OP>` from `mdmacros.h:56-58`, used only to document expectations in
/// assertion messages. Never used as the oracle -- the C `.so` is the oracle.
pub const INIT: c_int = if OP.as_bytes()[0] == b'm' { 1 } else { 0 };

pub fn config_label() -> String {
    format!("OP={OP} REPEAT={REPEAT}")
}

/* ------------------------------------------------------------------ *
 * Locating the artifacts.
 * ------------------------------------------------------------------ */

/// `translation/`
pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The working directory that holds `c_src/`, `translation/` and `cbuild/`.
pub fn work_root() -> PathBuf {
    manifest_dir().parent().expect("translation/ has a parent").to_path_buf()
}

/// `target/<profile>/` -- derived from the running test executable
/// (`target/<profile>/deps/<test>-<hash>`).
pub fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("test exe lives in target/<profile>/deps/")
        .to_path_buf()
}

fn require(path: PathBuf, what: &str, hint: &str) -> PathBuf {
    assert!(path.is_file(), "missing {what}: {} ({hint})", path.display());
    path
}

/// Path of the C shared library matching this test binary's configuration.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return require(PathBuf::from(p), "C .so from $C_SO", "check cbuild/build_c.sh");
    }
    require(
        work_root().join("cbuild").join(format!("libcmd_{OP}_{REPEAT}.so")),
        "C .so",
        "run cbuild/build_c.sh",
    )
}

/// Path of the Rust `cdylib`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return require(PathBuf::from(p), "Rust .so from $RUST_SO", "cargo build");
    }
    let dir = target_profile_dir();
    for name in ["libdriver.so", "driver.so"] {
        let cand = dir.join(name);
        if cand.is_file() {
            return cand;
        }
    }
    panic!("no libdriver.so in {} (cargo build --lib)", dir.display());
}

/// Path of the C `driver` executable matching this configuration.
pub fn c_driver_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER") {
        return require(PathBuf::from(p), "C driver from $C_DRIVER", "check cbuild/build_c.sh");
    }
    require(
        work_root().join("cbuild").join(format!("driver_{OP}_{REPEAT}")),
        "C driver",
        "run cbuild/build_c.sh",
    )
}

/// Path of the Rust `driver` executable.
pub fn rust_driver_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER") {
        return require(PathBuf::from(p), "Rust driver from $RUST_DRIVER", "cargo build");
    }
    let cand = target_profile_dir().join("driver");
    if cand.is_file() {
        return cand;
    }
    panic!("no driver executable in {} (cargo build --bin driver)", target_profile_dir().display());
}

/* ------------------------------------------------------------------ *
 * Loading.
 * ------------------------------------------------------------------ */

/// A loaded implementation, reached exclusively through its dynamic symbols.
pub struct Impl {
    pub name: &'static str,
    pub lib: Library,
}

impl Impl {
    fn open(name: &'static str, path: &Path) -> Impl {
        // Safety: the libraries are the artifacts built by this repo; loading
        // them runs their (trivial) initialisers.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
        Impl { name, lib }
    }

    pub fn binop(&self, sym: &str) -> Symbol<'_, BinOp> {
        let mut n = sym.as_bytes().to_vec();
        n.push(0);
        unsafe { self.lib.get(&n) }.unwrap_or_else(|e| panic!("{}: dlsym {sym}: {e}", self.name))
    }

    pub fn unop(&self, sym: &str) -> Symbol<'_, UnOp> {
        let mut n = sym.as_bytes().to_vec();
        n.push(0);
        unsafe { self.lib.get(&n) }.unwrap_or_else(|e| panic!("{}: dlsym {sym}: {e}", self.name))
    }

    /// Address of the exported `int (*G_OP)(int,int)` object.
    pub fn g_op(&self) -> *mut BinOp {
        let sym: Symbol<'_, *mut BinOp> =
            unsafe { self.lib.get(b"G_OP\0") }.unwrap_or_else(|e| panic!("{}: dlsym G_OP: {e}", self.name));
        *sym
    }

    /// Address of the exported `const char *G_OP_NAME` object.
    pub fn g_op_name(&self) -> *mut *const c_char {
        let sym: Symbol<'_, *mut *const c_char> = unsafe { self.lib.get(b"G_OP_NAME\0") }
            .unwrap_or_else(|e| panic!("{}: dlsym G_OP_NAME: {e}", self.name));
        *sym
    }

    /// The current NUL-terminated value of `G_OP_NAME`, as bytes.
    pub fn g_op_name_bytes(&self) -> Vec<u8> {
        let slot = self.g_op_name();
        unsafe {
            let p = *slot;
            assert!(!p.is_null(), "{}: G_OP_NAME is NULL", self.name);
            std::ffi::CStr::from_ptr(p).to_bytes().to_vec()
        }
    }
}

/// The C implementation.
pub fn c_impl() -> &'static Impl {
    static C: OnceLock<Impl> = OnceLock::new();
    C.get_or_init(|| Impl::open("C", &c_so_path()))
}

/// The Rust implementation (loaded as a `.so`, exactly like the C one).
pub fn rust_impl() -> &'static Impl {
    static R: OnceLock<Impl> = OnceLock::new();
    R.get_or_init(|| Impl::open("Rust", &rust_so_path()))
}

/* ------------------------------------------------------------------ *
 * Deterministic input generation (no external rand crate).
 * ------------------------------------------------------------------ */

/// `splitmix64`, seeded for reproducibility.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_9876;

impl Rng {
    pub fn new() -> Rng {
        Rng(SEED)
    }
    pub fn with_seed(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Full-range `i32`, biased towards small magnitudes a third of the time so
    /// both the "interesting small integer" and the "wide bit pattern" spaces
    /// are covered.
    pub fn next_i32(&mut self) -> c_int {
        let r = self.next_u64();
        match r % 3 {
            0 => ((r >> 8) as i16) as c_int,
            1 => (((r >> 8) as u32) % 64) as c_int - 32,
            _ => (r >> 32) as u32 as c_int,
        }
    }
    pub fn next_pair(&mut self) -> (c_int, c_int) {
        (self.next_i32(), self.next_i32())
    }
}

impl Default for Rng {
    fn default() -> Self {
        Rng::new()
    }
}

pub const RANDOM_CASES: usize = 256;

/// `(a, b)` shapes that are identities / tiny values.
pub const SMALL_PAIRS: &[(c_int, c_int)] = &[
    (0, 0),
    (0, 1),
    (1, 0),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
    (7, 3),
    (3, 7),
    (-5, 9),
    (9, -5),
    (2, -2),
    (0, -1),
    (-1, 0),
    (10, 0),
    (0, 10),
];

/// `(a, b)` shapes at / past the `int` boundaries (overflow in at least one op).
pub const BOUNDARY_PAIRS: &[(c_int, c_int)] = &[
    (c_int::MAX, 0),
    (0, c_int::MAX),
    (c_int::MIN, 0),
    (0, c_int::MIN),
    (c_int::MAX, 1),
    (1, c_int::MAX),
    (c_int::MIN, -1),
    (-1, c_int::MIN),
    (c_int::MIN, 1),
    (c_int::MAX, -1),
    (c_int::MAX, c_int::MAX),
    (c_int::MIN, c_int::MIN),
    (c_int::MAX, c_int::MIN),
    (c_int::MIN, c_int::MAX),
    (c_int::MAX, 2),
    (2, c_int::MAX),
    (65536, 65536),
    (-65536, 65536),
    (46341, 46341),
    (c_int::MAX / 2, 3),
];

/// Values of `n` that hit a distinct `DISPATCH_REP` `case`.
pub const IN_RANGE_N: &[c_int] = &[0, 1, 2, 3, 4, 5, 6];

/// Values of `n` that fall through to `default: break;`.
pub const OUT_OF_RANGE_N: &[c_int] =
    &[7, 8, 9, 10, 100, 1 << 20, 65536, -1, -2, -7, -100, c_int::MIN, c_int::MAX, c_int::MIN + 1, c_int::MAX - 1];

/* ------------------------------------------------------------------ *
 * Assertion helpers.
 * ------------------------------------------------------------------ */

/// Compare a one-argument export.
pub fn diff_unop(sym: &str, inputs: impl IntoIterator<Item = c_int>) {
    let (c, r) = (c_impl(), rust_impl());
    let (cf, rf) = (c.unop(sym), r.unop(sym));
    for n in inputs {
        let (cv, rv) = unsafe { (cf(n), rf(n)) };
        assert_eq!(
            cv, rv,
            "[{}] {sym}({n}): C returned {cv}, Rust returned {rv}",
            config_label()
        );
    }
}

/// Compare a two-argument export.
pub fn diff_binop(sym: &str, inputs: impl IntoIterator<Item = (c_int, c_int)>) {
    let (c, r) = (c_impl(), rust_impl());
    let (cf, rf) = (c.binop(sym), r.binop(sym));
    for (a, b) in inputs {
        let (cv, rv) = unsafe { (cf(a, b), rf(a, b)) };
        assert_eq!(
            cv, rv,
            "[{}] {sym}({a}, {b}): C returned {cv}, Rust returned {rv}",
            config_label()
        );
    }
}

/// `SMALL_PAIRS + BOUNDARY_PAIRS + RANDOM_CASES` seeded-random pairs.
pub fn all_pairs() -> Vec<(c_int, c_int)> {
    let mut v: Vec<(c_int, c_int)> = SMALL_PAIRS.iter().copied().collect();
    v.extend(BOUNDARY_PAIRS.iter().copied());
    let mut rng = Rng::new();
    for _ in 0..RANDOM_CASES {
        v.push(rng.next_pair());
    }
    v
}

pub fn random_pairs(seed: u64, n: usize) -> Vec<(c_int, c_int)> {
    let mut rng = Rng::with_seed(seed);
    (0..n).map(|_| rng.next_pair()).collect()
}

pub fn random_ints(seed: u64, n: usize) -> Vec<c_int> {
    let mut rng = Rng::with_seed(seed);
    (0..n).map(|_| rng.next_i32()).collect()
}

/* ------------------------------------------------------------------ *
 * Driver-executable differential runner (covers mdmain.c).
 * ------------------------------------------------------------------ */

/// Result of one process run.
#[derive(Debug, PartialEq, Eq)]
pub struct Run {
    pub status: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

fn run_one(exe: &Path, arg0: &str, args: &[&str]) -> Run {
    use std::os::unix::process::CommandExt;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg0(arg0);
    cmd.args(args);
    // Keep the environment from perturbing formatting/locale.
    cmd.env_remove("LC_ALL").env_remove("LANG").env("LC_ALL", "C");
    let out = cmd.output().unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    Run { status: out.status.code(), stdout: out.stdout, stderr: out.stderr }
}

/// Run the C `driver` and the Rust `driver` with an identical `argv` (including
/// `argv[0]`, set with `arg0` so the `usage:` line is byte-comparable) and
/// assert stdout, stderr and exit status all match exactly.
pub fn diff_driver(arg0: &str, args: &[&str]) {
    let c = run_one(&c_driver_path(), arg0, args);
    let r = run_one(&rust_driver_path(), arg0, args);
    let ctx = format!("[{}] argv0={arg0:?} args={args:?}", config_label());
    assert_eq!(
        c.stdout,
        r.stdout,
        "{ctx}: stdout differs\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "{ctx}: stderr differs\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(c.status, r.status, "{ctx}: exit status differs (C={:?} Rust={:?})", c.status, r.status);
}
