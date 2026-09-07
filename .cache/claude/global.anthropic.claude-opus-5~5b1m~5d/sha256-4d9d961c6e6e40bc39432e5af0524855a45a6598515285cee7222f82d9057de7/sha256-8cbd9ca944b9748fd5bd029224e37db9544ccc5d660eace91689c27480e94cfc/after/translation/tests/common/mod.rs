//! Shared plumbing for the differential tests.
//!
//! Every call goes through `libloading` into a `.so` — the Rust side is *never*
//! called directly, so the `#[no_mangle] extern "C"` wrappers are under test
//! too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

pub type JumpnodeFn = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;
pub type VoidFn = unsafe extern "C" fn();
pub type AddNodeFn = unsafe extern "C" fn(i32, i32, f64) -> i32;
pub type SetCountFn = unsafe extern "C" fn(i32);
pub type GetCountFn = unsafe extern "C" fn() -> i32;
pub type FindIdxFn = unsafe extern "C" fn(i32) -> i32;
pub type MetricFn = unsafe extern "C" fn(*const std::os::raw::c_char) -> i32;
pub type D2IFn = unsafe extern "C" fn(f64) -> i32;
pub type ProcBackFn = unsafe extern "C" fn(*mut i32, usize, i32) -> i32;
pub type GetNodeFn = unsafe extern "C" fn(i32, *mut i32, *mut i32, *mut f64, *mut i32) -> i32;
pub type IntFn = unsafe extern "C" fn() -> i32;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

/// The C shared library built by `c_src/CMakeLists.txt`. Its name is derived
/// from the parent directory name, so glob for it instead of hardcoding.
pub fn c_so_path() -> PathBuf {
    let dir = repo_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("c_src/build not readable ({e}); build the C lib first: \
             cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with("lib") && s.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert!(!found.is_empty(), "no lib*.so found in {}", dir.display());
    found.remove(0)
}

/// The Rust `cdylib`. Overridable with `RUST_SO=` so the same tests can be run
/// against the debug and release artifacts.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    let base = repo_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libjumpnode_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "libjumpnode_lib.so not found under {}; run `cargo build --release`",
        base.display()
    );
}

/// The instrumented build of the *original C* (`tests/c_harness/harness.c`
/// `#include`s `c_src/src/lib.c` verbatim).
pub fn c_harness_so_path() -> PathBuf {
    let p = repo_root().join("translation/tests/c_harness/build/libharness.so");
    assert!(
        p.exists(),
        "missing {}; build it with ./build_harness.sh",
        p.display()
    );
    p
}

pub struct Pair {
    pub c: Library,
    pub rs: Library,
}

impl Pair {
    /// Loads the *shipped* C `.so` and the Rust `cdylib`.
    ///
    /// The shipped C `.so` can never have a non-zero `node_count`
    /// (`initialize_test_data` is `static` and unreferenced), so the Rust side is
    /// forced back to that same baseline when the test hooks are compiled in —
    /// otherwise a state-mutating test elsewhere in the binary would leak into
    /// these comparisons.
    pub fn shipped() -> Pair {
        let p = unsafe {
            Pair {
                c: Library::new(c_so_path()).expect("load C .so"),
                rs: Library::new(rust_so_path()).expect("load Rust .so"),
            }
        };
        if let Ok(set) = unsafe { p.rs.get::<SetCountFn>(b"jumpnode_test_set_node_count\0") } {
            unsafe { set(0) };
        }
        p
    }

    /// Loads the instrumented C harness and the Rust `cdylib`.
    /// Returns `None` when the Rust `.so` lacks the feature-gated hooks.
    pub fn instrumented() -> Option<Pair> {
        let p = unsafe {
            Pair {
                c: Library::new(c_harness_so_path()).expect("load C harness .so"),
                rs: Library::new(rust_so_path()).expect("load Rust .so"),
            }
        };
        let has: Result<Symbol<VoidFn>, _> =
            unsafe { p.rs.get(b"jumpnode_initialize_test_data\0") };
        if has.is_err() { None } else { Some(p) }
    }

    pub fn sym<T>(&self, name: &[u8]) -> (Symbol<'_, T>, Symbol<'_, T>) {
        let c = unsafe { self.c.get::<T>(name) }
            .unwrap_or_else(|e| panic!("C .so missing {}: {e}", String::from_utf8_lossy(name)));
        let rs = unsafe { self.rs.get::<T>(name) }
            .unwrap_or_else(|e| panic!("Rust .so missing {}: {e}", String::from_utf8_lossy(name)));
        (c, rs)
    }
}

/// Serializes tests that mutate the libraries' process-global state.
///
/// `dlopen`ing the same path twice returns the *same* handle, so `node_storage`
/// / `node_count` are shared by every test in a binary. Without this lock,
/// concurrently running tests interleave their mutations between the C call and
/// the Rust call and report bogus divergences.
pub static STATE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn lock_state() -> std::sync::MutexGuard<'static, ()> {
    match STATE_LOCK.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Deterministic xorshift64* PRNG — fixed seed keeps every run reproducible.
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
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// A mix of small, boundary and fully random values.
    pub fn interesting_i32(&mut self) -> i32 {
        match self.next_u64() % 8 {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MAX,
            4 => i32::MIN,
            5 => self.range(-100, 100),
            6 => self.range(-1_000_000, 1_000_000),
            _ => self.next_i32(),
        }
    }
}

/// The four `int` args of `jumpnode`, for readable failure messages.
pub const SEED: u64 = 0x5EED_1234;

#[track_caller]
pub fn assert_eq_call(
    label: &str,
    c: &Symbol<'_, JumpnodeFn>,
    rs: &Symbol<'_, JumpnodeFn>,
    a: i32,
    b: i32,
    d: i32,
    e: i32,
) {
    let got_c = unsafe { c(a, b, d, e) };
    let got_rs = unsafe { rs(a, b, d, e) };
    assert_eq!(
        got_c, got_rs,
        "[{label}] jumpnode({a}, {b}, {d}, {e}): C returned {got_c}, Rust returned {got_rs}"
    );
}
