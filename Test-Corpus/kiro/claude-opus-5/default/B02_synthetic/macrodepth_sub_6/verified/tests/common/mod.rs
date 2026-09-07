//! Shared differential-testing harness.
//!
//! Loads BOTH shared libraries through `libloading` and calls only their
//! exported symbols, so the `#[no_mangle]`/`extern "C"` wrappers are part of
//! what is under test. Nothing in this module calls a `driver` function
//! directly.
//!
//! The C `.so` is built from `c_src/src/mdcore.c` with an equivalent PIC
//! compile (`c_src/CMakeLists.txt` only declares `add_executable`, so CMake
//! alone never produces a `.so`); the Rust `.so` is the crate's `cdylib`. Both
//! are (re)built on demand for whatever feature set the test binary was
//! compiled with, into a side target directory so the outer `cargo test`
//! lock is not contended.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::ffi::c_char;
use std::ffi::c_int;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// Which configuration is this test binary compiled for?
// ---------------------------------------------------------------------------
//
// Mirrors the cfg precedence in `src/mdmacros.rs` exactly, so the harness and
// the library under test can never disagree about what `-DOP`/`-DREPEAT` the C
// side should be built with.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op {
    Add,
    Sub,
    Mul,
}

impl Op {
    pub fn cmake_value(self) -> &'static str {
        match self {
            Op::Add => "add",
            Op::Sub => "sub",
            Op::Mul => "mul",
        }
    }

    /// `INIT_FOR(OP)`.
    pub fn init(self) -> c_int {
        match self {
            Op::Mul => 1,
            _ => 0,
        }
    }

    /// `OP_FN(OP)` applied, with the C build's two's-complement wraparound.
    pub fn apply(self, a: c_int, b: c_int) -> c_int {
        match self {
            Op::Add => a.wrapping_add(b),
            Op::Sub => a.wrapping_sub(b),
            Op::Mul => a.wrapping_mul(b),
        }
    }

    /// `STEP_<OP>(acc, i)`.
    pub fn step(self, acc: c_int, i: c_int) -> c_int {
        match self {
            Op::Add => acc.wrapping_add(i),
            Op::Sub => acc.wrapping_sub(i),
            Op::Mul => acc.wrapping_mul(i.wrapping_add(1)),
        }
    }
}

#[cfg(feature = "mul")]
pub const OP: Op = Op::Mul;
#[cfg(all(feature = "sub", not(feature = "mul")))]
pub const OP: Op = Op::Sub;
#[cfg(all(not(feature = "sub"), not(feature = "mul")))]
pub const OP: Op = Op::Add;

#[cfg(feature = "0")]
pub const REPEAT: c_int = 0;
#[cfg(all(feature = "1", not(feature = "0")))]
pub const REPEAT: c_int = 1;
#[cfg(all(feature = "2", not(any(feature = "0", feature = "1"))))]
pub const REPEAT: c_int = 2;
#[cfg(all(feature = "3", not(any(feature = "0", feature = "1", feature = "2"))))]
pub const REPEAT: c_int = 3;
#[cfg(all(
    feature = "4",
    not(any(feature = "0", feature = "1", feature = "2", feature = "3"))
))]
pub const REPEAT: c_int = 4;
#[cfg(all(
    feature = "6",
    not(any(
        feature = "0",
        feature = "1",
        feature = "2",
        feature = "3",
        feature = "4"
    ))
))]
pub const REPEAT: c_int = 6;
#[cfg(all(
    feature = "7",
    not(any(
        feature = "0",
        feature = "1",
        feature = "2",
        feature = "3",
        feature = "4",
        feature = "6"
    ))
))]
pub const REPEAT: c_int = 7;
#[cfg(not(any(
    feature = "0",
    feature = "1",
    feature = "2",
    feature = "3",
    feature = "4",
    feature = "6",
    feature = "7"
)))]
pub const REPEAT: c_int = 5;

/// `cbuild/` artifact tag, e.g. `mul_3`.
pub fn tag() -> String {
    format!("{}_{}", OP.cmake_value(), REPEAT)
}

/// `REP<n>(OP, acc)` — the reference unrolling, independent of the translation.
pub fn rep_reference(n: c_int) -> c_int {
    let mut acc = OP.init();
    let mut i = 0;
    while i < n {
        acc = OP.step(acc, i);
        i += 1;
    }
    acc
}

/// `DISPATCH_REP(OP, acc, n)` — the reference `switch`, cases 0..=6 only.
pub fn dispatch_reference(n: c_int) -> c_int {
    match n {
        0..=6 => rep_reference(n),
        _ => OP.init(),
    }
}

// ---------------------------------------------------------------------------
// Build / locate the two shared libraries
// ---------------------------------------------------------------------------

/// Repository root (the directory holding `c_src/` and `translation/`).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn artifacts() -> &'static (PathBuf, PathBuf) {
    static ARTIFACTS: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();
    ARTIFACTS.get_or_init(|| {
        let root = repo_root();
        let tag = tag();
        let out = root.join("cbuild/harness");
        std::fs::create_dir_all(&out).expect("create cbuild/harness");

        // --- C shared library from c_src/src/mdcore.c (c_src/ is never touched) ---
        let c_so = match std::env::var_os("MD_C_SO") {
            Some(p) => PathBuf::from(p),
            None => {
                let c_so = out.join(format!("libmdcore_{tag}.so"));
                let status = Command::new("gcc")
                    .args(["-O2", "-fPIC", "-std=c11"])
                    .arg(format!("-DOP={}", OP.cmake_value()))
                    .arg(format!("-DREPEAT={REPEAT}"))
                    .arg("-shared")
                    .arg("-o")
                    .arg(&c_so)
                    .arg(root.join("c_src/src/mdcore.c"))
                    .status()
                    .expect("run gcc");
                assert!(status.success(), "gcc failed to build the C .so for {tag}");
                c_so
            }
        };

        // --- Rust cdylib for the same feature set ---
        let rust_so = match std::env::var_os("MD_RUST_SO") {
            Some(p) => PathBuf::from(p),
            None => {
                let target_dir = root.join("cbuild/harness-target");
                let status = Command::new(env!("CARGO"))
                    .current_dir(root.join("translation"))
                    .env("CARGO_TARGET_DIR", &target_dir)
                    // Do not inherit the outer test run's cfg/feature env.
                    .env_remove("RUSTFLAGS")
                    .args([
                        "build",
                        "--release",
                        "--lib",
                        "--no-default-features",
                        "--features",
                    ])
                    .arg(format!("{},{}", OP.cmake_value(), REPEAT))
                    .status()
                    .expect("run cargo build for the cdylib");
                assert!(status.success(), "cargo failed to build the Rust .so for {tag}");
                let built = target_dir.join("release/libdriver.so");
                let stamped = out.join(format!("libdriver_{tag}.so"));
                std::fs::copy(&built, &stamped).expect("stash the Rust .so");
                stamped
            }
        };

        (c_so, rust_so)
    })
}

pub fn c_so_path() -> &'static Path {
    &artifacts().0
}

pub fn rust_so_path() -> &'static Path {
    &artifacts().1
}

/// The two `driver` executables (C: `mdcore.c` + `mdmain.c`; Rust: the `[[bin]]`).
pub fn driver_paths() -> &'static (PathBuf, PathBuf) {
    static DRIVERS: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();
    DRIVERS.get_or_init(|| {
        let root = repo_root();
        let tag = tag();
        let out = root.join("cbuild/harness");
        std::fs::create_dir_all(&out).expect("create cbuild/harness");

        let c_exe = out.join(format!("driver_c_{tag}"));
        let status = Command::new("gcc")
            .args(["-O2", "-std=c11"])
            .arg(format!("-DOP={}", OP.cmake_value()))
            .arg(format!("-DREPEAT={REPEAT}"))
            .arg("-o")
            .arg(&c_exe)
            .arg(root.join("c_src/src/mdcore.c"))
            .arg(root.join("c_src/src/mdmain.c"))
            .status()
            .expect("run gcc");
        assert!(status.success(), "gcc failed to build the C driver for {tag}");

        // Force the cdylib+bin build to have happened, then take the bin.
        let _ = artifacts();
        let target_dir = root.join("cbuild/harness-target");
        let status = Command::new(env!("CARGO"))
            .current_dir(root.join("translation"))
            .env("CARGO_TARGET_DIR", &target_dir)
            .env_remove("RUSTFLAGS")
            .args([
                "build",
                "--release",
                "--bin",
                "driver",
                "--no-default-features",
                "--features",
            ])
            .arg(format!("{},{}", OP.cmake_value(), REPEAT))
            .status()
            .expect("run cargo build for the driver bin");
        assert!(status.success(), "cargo failed to build the Rust driver for {tag}");
        let rust_exe = out.join(format!("driver_rust_{tag}"));
        std::fs::copy(target_dir.join("release/driver"), &rust_exe).expect("stash the Rust driver");

        (c_exe, rust_exe)
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every libc stream, which is how the C `.so`'s
    /// `printf` buffer is drained (the `.so` shares this process's libc).
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

/// Serializes fd-1 redirection; `cargo test` runs tests in parallel threads.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Runs `body` with fd 1 pointed at a temporary file and returns the bytes it
/// wrote. Both implementations are measured the same way.
pub fn capture_stdout<R>(body: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let _guard = capture_lock();

    let mut path = std::env::temp_dir();
    path.push(format!(
        "md_cap_{}_{}_{:?}.txt",
        std::process::id(),
        tag(),
        std::thread::current().id()
    ));

    let value;
    let mut bytes = Vec::new();
    unsafe {
        // Drain anything already buffered on either side before swapping fds.
        fflush(std::ptr::null_mut());
        let file = std::fs::File::create(&path).expect("create capture file");
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = fd_of(&file);
        assert!(dup2(fd, 1) >= 0, "dup2 onto stdout failed");
        drop(file);

        value = body();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring stdout failed");
        close(saved);
    }
    std::fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut bytes)
        .expect("read capture file");
    let _ = std::fs::remove_file(&path);
    (value, bytes)
}

fn fd_of(file: &std::fs::File) -> c_int {
    use std::os::unix::io::AsRawFd;
    file.as_raw_fd()
}

// ---------------------------------------------------------------------------
// The library under test, behind its exported symbols only
// ---------------------------------------------------------------------------

pub type BinFn = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type UnFn = unsafe extern "C" fn(c_int) -> c_int;

/// One loaded shared library, reachable only via `nm -D` symbol names.
pub struct Lib {
    pub name: &'static str,
    _lib: libloading::Library,
    pub op_add: BinFn,
    pub op_sub: BinFn,
    pub op_mul: BinFn,
    pub helper_call: BinFn,
    pub helper_ptr: BinFn,
    pub use_generated: UnFn,
    /// `int (*G_OP)(int,int)` — the exported data slot itself.
    pub g_op: *mut BinFn,
    /// `const char *G_OP_NAME` — the exported data slot itself.
    pub g_op_name: *mut *const c_char,
}

impl Lib {
    fn open(name: &'static str, path: &Path) -> Lib {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen {} ({}): {e}", path.display(), name));
            macro_rules! sym {
                ($t:ty, $s:literal) => {{
                    let s: libloading::Symbol<$t> = lib
                        .get($s)
                        .unwrap_or_else(|e| panic!("{name}: missing symbol {}: {e}", String::from_utf8_lossy(&$s[..$s.len() - 1])));
                    *s
                }};
            }
            // For a data symbol, `libloading` reinterprets the symbol's own
            // address as the requested type, so asking for `*mut T` yields a
            // pointer to the exported slot. Copying it out of the `Symbol`
            // immediately keeps no borrow on `lib`.
            macro_rules! data {
                ($t:ty, $s:literal) => {{
                    let s: libloading::Symbol<*mut $t> = lib
                        .get($s)
                        .unwrap_or_else(|e| panic!("{name}: missing data symbol {}: {e}", String::from_utf8_lossy(&$s[..$s.len() - 1])));
                    *s
                }};
            }
            let op_add = sym!(BinFn, b"op_add\0");
            let op_sub = sym!(BinFn, b"op_sub\0");
            let op_mul = sym!(BinFn, b"op_mul\0");
            let helper_call = sym!(BinFn, b"helper_call\0");
            let helper_ptr = sym!(BinFn, b"helper_ptr\0");
            let use_generated = sym!(UnFn, b"use_generated\0");
            let g_op = data!(BinFn, b"G_OP\0");
            let g_op_name = data!(*const c_char, b"G_OP_NAME\0");
            Lib {
                name,
                _lib: lib,
                op_add,
                op_sub,
                op_mul,
                helper_call,
                helper_ptr,
                use_generated,
                g_op,
                g_op_name,
            }
        }
    }

    /// The `G_OP` value currently stored in the library's data segment.
    pub fn load_g_op(&self) -> BinFn {
        unsafe { std::ptr::read(self.g_op) }
    }

    pub fn store_g_op(&self, f: BinFn) {
        unsafe { std::ptr::write(self.g_op, f) }
    }

    /// `G_OP_NAME`'s NUL-terminated bytes (excluding the NUL).
    pub fn g_op_name_bytes(&self) -> Vec<u8> {
        unsafe {
            let p = std::ptr::read(self.g_op_name);
            assert!(!p.is_null(), "{}: G_OP_NAME is NULL", self.name);
            let mut out = Vec::new();
            let mut i = 0isize;
            loop {
                let b = *p.offset(i) as u8;
                if b == 0 {
                    break;
                }
                out.push(b);
                i += 1;
                assert!(i < 64, "{}: G_OP_NAME is not NUL-terminated", self.name);
            }
            out
        }
    }
}

/// The C and Rust libraries, opened once per test binary.
pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| Pair {
        c: Lib::open("C", c_so_path()),
        rust: Lib::open("Rust", rust_so_path()),
    })
}

// SAFETY: both libraries are opened once and never unloaded; the raw data
// pointers stay valid for the process lifetime. Concurrent access to `G_OP` is
// serialized by the tests that write it.
unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

/// One observation of a call: the returned `int` plus the bytes it printed.
#[derive(PartialEq, Eq, Debug)]
pub struct Obs {
    pub ret: c_int,
    pub out: Vec<u8>,
}

impl std::fmt::Display for Obs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ret={} stdout={:?}",
            self.ret,
            String::from_utf8_lossy(&self.out)
        )
    }
}

pub fn observe(f: impl FnOnce() -> c_int) -> Obs {
    let (ret, out) = capture_stdout(f);
    Obs { ret, out }
}

/// Calls the same binary entry point in both libraries and asserts the return
/// value and the printed bytes match exactly.
pub fn diff_bin(what: &str, pick: impl Fn(&Lib) -> BinFn, a: c_int, b: c_int) -> c_int {
    let p = pair();
    let cf = pick(&p.c);
    let rf = pick(&p.rust);
    let c = observe(|| unsafe { cf(a, b) });
    let r = observe(|| unsafe { rf(a, b) });
    assert_eq!(
        c, r,
        "[{}] {what}({a}, {b}) diverged\n  C:    {c}\n  Rust: {r}",
        tag()
    );
    c.ret
}

/// Same, for the single-argument entry point.
pub fn diff_un(what: &str, pick: impl Fn(&Lib) -> UnFn, n: c_int) -> c_int {
    let p = pair();
    let cf = pick(&p.c);
    let rf = pick(&p.rust);
    let c = observe(|| unsafe { cf(n) });
    let r = observe(|| unsafe { rf(n) });
    assert_eq!(
        c, r,
        "[{}] {what}({n}) diverged\n  C:    {c}\n  Rust: {r}",
        tag()
    );
    c.ret
}

/// Calls through the `G_OP` data symbol in both libraries.
pub fn diff_g_op(a: c_int, b: c_int) -> c_int {
    let p = pair();
    let cf = p.c.load_g_op();
    let rf = p.rust.load_g_op();
    let c = observe(|| unsafe { cf(a, b) });
    let r = observe(|| unsafe { rf(a, b) });
    assert_eq!(
        c, r,
        "[{}] G_OP({a}, {b}) diverged\n  C:    {c}\n  Rust: {r}",
        tag()
    );
    c.ret
}

// ---------------------------------------------------------------------------
// Deterministic inputs
// ---------------------------------------------------------------------------

/// SplitMix64 — fixed seed, so every run and every configuration sees the same
/// sequence and a failure is reproducible.
pub struct Rng(u64);

pub const SEED: u64 = 0x5DEE_CE66D;

impl Rng {
    pub fn new() -> Rng {
        Rng(SEED)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_i32(&mut self) -> c_int {
        self.next_u64() as u32 as c_int
    }

    /// A mix of full-range values and small values, so both wraparound-prone
    /// and ordinary arithmetic get exercised.
    pub fn next_operand(&mut self) -> c_int {
        let w = self.next_u64();
        match w % 4 {
            0 => (w >> 8) as u32 as c_int,
            1 => ((w >> 8) as i32) % 256,
            2 => ((w >> 8) as i32) % 65_536,
            _ => (w >> 8) as u32 as c_int,
        }
    }
}

/// How many randomized cases each property-style row runs.
pub const CASES: usize = 512;

/// Fixed boundary values, crossed with themselves by the boundary rows.
pub const BOUNDS: &[c_int] = &[
    0,
    1,
    -1,
    2,
    -2,
    7,
    -7,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    0x7FFF,
    -0x8000,
    65_535,
    65_536,
];

/// `nm -D --defined-only` names, for the symbol-parity test.
pub fn defined_dynamic_symbols(path: &Path) -> BTreeSet<(String, String)> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let _addr = it.next()?;
            let class = it.next()?.to_string();
            let name = it.next()?.to_string();
            Some((name, class))
        })
        .collect()
}
