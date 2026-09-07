//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries with `libloading` and calls the exported
//! `memchra2` symbol through the FFI boundary in each. The Rust implementation
//! is NEVER called directly — only via its `cdylib` export, so the
//! `#[no_mangle] extern "C"` wrapper is under test too.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;
use std::sync::OnceLock;

pub type Memchra2Fn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&build) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(p);
            }
        }
    }
    assert!(
        !candidates.is_empty(),
        "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build.display()
    );
    candidates.sort();
    candidates.remove(0)
}

pub fn find_rust_so() -> PathBuf {
    // Explicit override, so the very same suite can be pointed at the debug
    // cdylib (overflow checks ON, unwinding panics) as well as the release one.
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "HARVEST_RUST_SO={} is not a file", p.display());
        return p;
    }
    // Prefer the profile the tests were built with, but accept either.
    let target = workspace_root().join("translation").join("target");
    let mut tried = Vec::new();
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libmemchra2_lib.so");
        if p.is_file() {
            return p;
        }
        tried.push(p);
    }
    panic!(
        "no Rust cdylib found; looked at {:?}. Build it with `cargo build --release`.",
        tried
    );
}

struct Libs {
    c: Library,
    rs: Library,
}

// Safety: the libraries are leaked for the process lifetime and both are pure
// leaf functions with no global state.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rs_path = find_rust_so();
        eprintln!("C  .so: {}", c_path.display());
        eprintln!("RS .so: {}", rs_path.display());
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
        let rs = unsafe { Library::new(&rs_path) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", rs_path.display()));
        Libs { c, rs }
    })
}

/// The two `memchra2` entry points, resolved from the two `.so`s.
pub struct Pair {
    pub c: Symbol<'static, Memchra2Fn>,
    pub rs: Symbol<'static, Memchra2Fn>,
}

pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let l = libs();
        let c: Symbol<Memchra2Fn> = unsafe { l.c.get(b"memchra2\0") }
            .expect("C .so does not export `memchra2`");
        let rs: Symbol<Memchra2Fn> = unsafe { l.rs.get(b"memchra2\0") }
            .expect("Rust .so does not export `memchra2` (missing #[no_mangle] wrapper?)");
        Pair { c, rs }
    })
}

/// Calls both implementations and asserts bit-identical `int` results.
#[track_caller]
pub fn check(a: i32, b: i32, c: i32, d: i32) {
    let p = pair();
    let got_c = unsafe { (p.c)(a, b, c, d) };
    let got_rs = unsafe { (p.rs)(a, b, c, d) };
    assert_eq!(
        got_c, got_rs,
        "divergence at memchra2({a}, {b}, {c}, {d})\n  \
         C   = {got_c} (0x{got_c:08x})\n  \
         RS  = {got_rs} (0x{got_rs:08x})\n  \
         hex args: a=0x{a:08x} b=0x{b:08x} c=0x{c:08x} d=0x{d:08x}\n  \
         a as f32 = {:?}",
        f32::from_bits(a as u32),
    );
}

/// Deterministic xorshift64* PRNG — reproducible across runs and platforms.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
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
    /// Uniform in `[lo, hi]` inclusive (as `u32` range), returned as `i32` bits.
    pub fn u32_in(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }
    /// Uniform `i32` in an inclusive signed range.
    pub fn i32_in(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64) as u64 + 1;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

/// Interesting `int` values reused by several rows.
pub const EXTREMES: &[i32] = &[
    i32::MIN,
    i32::MIN + 1,
    -2_000_000_000,
    -1_000_000_001,
    -1_000_000_000,
    -999_999_999,
    -100_001,
    -100_000,
    -10_000,
    -1_000,
    -1_000_000_000 / 1_000,
    -999,
    -100,
    -99,
    -10,
    -9,
    -2,
    -1,
    0,
    1,
    2,
    9,
    10,
    99,
    100,
    999,
    1_000,
    10_000,
    99_999,
    100_000,
    999_999_999,
    1_000_000_000,
    2_000_000_000,
    i32::MAX - 1,
    i32::MAX,
];
