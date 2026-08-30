//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! driven purely through their exported C-ABI symbols, so the `#[no_mangle]`
//! wrappers are part of what is under test.
//!
//! * C reference: `/tmp/cref/<op>_<repeat>/libmdcore.so`, produced by
//!   `build_cref.sh` from the pristine `c_src/src/mdcore.c`.
//! * Rust:        `target/<profile>/libdriver.so`, the crate's `cdylib`.
//!
//! The active `<op>_<repeat>` directory is selected with the same `cfg`
//! cascade the translation itself uses, so `cargo test --features sub,3`
//! automatically compares against the `-DOP=sub -DREPEAT=3` C build.

#![allow(dead_code)]

use std::ffi::{CStr, OsStr, c_char, c_int};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/* ------------------------------------------------------------------ config */

/// `OP`, matching the precedence documented in `Cargo.toml` (mul > sub > add,
/// `add` when nothing is selected — the `#ifndef OP` fallback).
pub const OP: &str = if cfg!(feature = "mul") {
    "mul"
} else if cfg!(feature = "sub") {
    "sub"
} else {
    "add"
};

/// `REPEAT`: lowest enabled value wins, 5 when nothing is selected.
pub const REPEAT: c_int = if cfg!(feature = "0") {
    0
} else if cfg!(feature = "1") {
    1
} else if cfg!(feature = "2") {
    2
} else if cfg!(feature = "3") {
    3
} else if cfg!(feature = "4") {
    4
} else if cfg!(feature = "5") {
    5
} else if cfg!(feature = "6") {
    6
} else if cfg!(feature = "7") {
    7
} else {
    5
};

/// `INIT_FOR(OP)`.
pub const INIT: c_int = if cfg!(feature = "mul") { 1 } else { 0 };

pub fn config_name() -> String {
    format!("{OP}_{REPEAT}")
}

/* ------------------------------------------------------------------- paths */

/// `target/<profile>/` — the test binary lives in `<profile>/deps/`.
pub fn artifact_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

pub fn rust_so_path() -> PathBuf {
    artifact_dir().join("libdriver.so")
}

pub fn rust_bin_path() -> PathBuf {
    artifact_dir().join("driver")
}

pub fn c_ref_dir() -> PathBuf {
    PathBuf::from("/tmp/cref").join(config_name())
}

pub fn c_so_path() -> PathBuf {
    c_ref_dir().join("libmdcore.so")
}

pub fn c_bin_path() -> PathBuf {
    c_ref_dir().join("driver")
}

fn require(p: &Path, how: &str) -> PathBuf {
    assert!(p.exists(), "missing {}; build it with: {how}", p.display());
    p.to_path_buf()
}

/* ------------------------------------------------------------------ loader */

/// One loaded implementation, addressed only through its dynamic symbols.
pub struct Impl {
    pub name: &'static str,
    lib: Library,
}

impl Impl {
    fn open(name: &'static str, path: &Path) -> Impl {
        // SAFETY: both objects are plain C-ABI libraries built from this repo.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
        Impl { name, lib }
    }

    pub fn c() -> Impl {
        let p = require(
            &c_so_path(),
            "./build_cref.sh (from the working-directory root)",
        );
        Impl::open("C", &p)
    }

    pub fn rust() -> Impl {
        let p = require(
            &rust_so_path(),
            "cargo build --no-default-features --features <combo>",
        );
        Impl::open("Rust", &p)
    }

    /// Both implementations, C first.
    pub fn pair() -> (Impl, Impl) {
        (Impl::c(), Impl::rust())
    }

    fn sym<T>(&self, name: &str) -> Symbol<'_, T> {
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(0);
        // SAFETY: the caller states the symbol's C type; every use below matches
        // the prototypes in `c_src/src/mdmacros.h`.
        unsafe { self.lib.get::<T>(&bytes) }
            .unwrap_or_else(|e| panic!("{}: dlsym {name}: {e}", self.name))
    }

    /// `int f(int, int)` — `op_add` / `op_sub` / `op_mul` / `helper_*`.
    pub fn fn2(&self, name: &str) -> extern "C" fn(c_int, c_int) -> c_int {
        *self.sym::<extern "C" fn(c_int, c_int) -> c_int>(name)
    }

    /// `int f(int)` — `use_generated`.
    pub fn fn1(&self, name: &str) -> extern "C" fn(c_int) -> c_int {
        *self.sym::<extern "C" fn(c_int) -> c_int>(name)
    }

    /// Reads the `int (*G_OP)(int,int)` data symbol and returns its value.
    pub fn g_op(&self) -> extern "C" fn(c_int, c_int) -> c_int {
        // SAFETY: `G_OP` is an initialised function-pointer object.
        unsafe { *self.g_op_slot() }
    }

    /// Address of the `G_OP` object itself. `mdmacros.h` declares it as a plain
    /// `extern int (*G_OP)(int,int);`, i.e. a writable global.
    pub fn g_op_slot(&self) -> *mut extern "C" fn(c_int, c_int) -> c_int {
        *self.sym::<*mut extern "C" fn(c_int, c_int) -> c_int>("G_OP")
    }

    /// Address of the `G_OP_NAME` object itself (also writable in C).
    pub fn g_op_name_slot(&self) -> *mut *const c_char {
        *self.sym::<*mut *const c_char>("G_OP_NAME")
    }

    /// Reads the `const char *G_OP_NAME` data symbol as raw bytes.
    pub fn g_op_name(&self) -> Vec<u8> {
        // SAFETY: `G_OP_NAME` points at a static NUL-terminated string.
        unsafe { CStr::from_ptr(*self.g_op_name_slot()) }
            .to_bytes()
            .to_vec()
    }

    /// True when the symbol is resolvable at all (used for export coverage).
    pub fn has_symbol(&self, name: &str) -> bool {
        let mut bytes = name.as_bytes().to_vec();
        bytes.push(0);
        unsafe { self.lib.get::<*const ()>(&bytes) }.is_ok()
    }
}

/* --------------------------------------------------------- stdout capture */

/// Runs `f` with file descriptor 1 redirected into a temporary file and returns
/// its result together with the exact bytes written.
///
/// `printf` in the C object and `println!` in the Rust object both land on fd 1,
/// so this is the only way to compare their output byte-for-byte from outside.
pub fn capture_stdout<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let path = std::env::temp_dir().join(format!(
        "mdcap-{}-{:?}-{}.txt",
        std::process::id(),
        std::thread::current().id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .expect("open capture file");

    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Flush anything already pending so it is not attributed to `f`.
    let _ = std::io::stdout().flush();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    let target = std::os::unix::io::AsRawFd::as_raw_fd(&file);
    assert!(unsafe { libc::dup2(target, 1) } >= 0, "dup2 failed");

    let out = f();

    // The C object's stdout is fully buffered when fd 1 is a file; Rust's
    // LineWriter flushes on newline but flush both to be certain.
    let _ = std::io::stdout().flush();
    unsafe { libc::fflush(std::ptr::null_mut()) };
    unsafe {
        libc::dup2(saved, 1);
        libc::close(saved);
    }

    let mut buf = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("seek");
    file.read_to_end(&mut buf).expect("read capture");
    drop(file);
    let _ = std::fs::remove_file(&path);
    (out, buf)
}

static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/* --------------------------------------------------------------- test data */

/// Operands covering zero, signs, small values and the extremes where the C
/// arithmetic wraps around.
pub fn operands() -> Vec<c_int> {
    vec![
        0,
        1,
        -1,
        2,
        -2,
        3,
        7,
        -7,
        10,
        -10,
        100,
        -100,
        12345,
        -12345,
        65535,
        65536,
        1 << 16,
        1 << 30,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        46341,
        -46341,
        1_000_000_000,
        -1_000_000_000,
    ]
}

/// A smaller cross-product so the O(n^2) sweeps stay fast.
pub fn operand_pairs() -> Vec<(c_int, c_int)> {
    let v = operands();
    let mut out = Vec::new();
    for &a in &v {
        for &b in &v {
            out.push((a, b));
        }
    }
    out
}

/// Arguments handed to `use_generated`, including the `default:` cases of
/// `DISPATCH_REP` (anything outside `0..=6`).
pub fn accum_inputs() -> Vec<c_int> {
    let mut v: Vec<c_int> = (-4..=12).collect();
    v.extend([100, -100, i32::MAX, i32::MIN, REPEAT]);
    v
}

/// Argument vectors for the `driver` executable comparison.
pub fn driver_args() -> Vec<Vec<&'static OsStr>> {
    [
        vec!["7", "3"],
        vec!["0", "0"],
        vec!["-5", "9"],
        vec!["3", "-4"],
        vec!["1", "1"],
        vec!["2147483647", "2"],
        vec!["-2147483648", "-1"],
        vec!["  -12abc", "+9"],
        vec!["99999999999999999999", "3"],
        vec!["12x", "7"],
        vec!["", ""],
        vec!["+0", "-0"],
        vec!["0x10", "010"],
        vec!["   \t 42", "-0000000009"],
        vec!["2147483648", "-2147483649"],
        vec!["4294967296", "4294967297"],
        vec!["-", "+"],
        vec!["7", "3", "extra"],
    ]
    .into_iter()
    .map(|v| v.into_iter().map(OsStr::new).collect())
    .collect()
}
