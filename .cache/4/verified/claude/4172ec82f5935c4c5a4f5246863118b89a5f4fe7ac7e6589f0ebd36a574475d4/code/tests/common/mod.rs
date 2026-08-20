//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and every
//! call goes through the `jumpnode` dynamic symbol. The Rust implementation is
//! *never* called directly, so the `#[unsafe(no_mangle)] extern "C"` export
//! wrapper is exercised exactly as an external consumer would exercise it.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type JumpnodeFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/CMakeLists.txt` does not link `m`, so the C `.so` has an unresolved
/// `U sqrt`. Publish libm globally before opening it so `RTLD_NOW` resolves.
fn preload_libm() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        #[cfg(unix)]
        {
            use libloading::os::unix::{Library as UnixLibrary, RTLD_GLOBAL, RTLD_NOW};
            for name in ["libm.so.6", "libm.so"] {
                if let Ok(lib) = unsafe { UnixLibrary::open(Some(name), RTLD_NOW | RTLD_GLOBAL) } {
                    // Leak on purpose: the symbols must stay resolvable forever.
                    std::mem::forget(lib);
                    break;
                }
            }
        }
    });
}

/// Configure+build a CMake project into `<dir>/build` if the expected `.so` is
/// not already there, so a bare `cargo test` works from a clean checkout.
fn ensure_cmake_built(src_dir: &Path, lib_name: &str) -> Option<PathBuf> {
    let build = src_dir.join("build");
    let so = build.join(lib_name);
    if so.is_file() {
        return Some(so);
    }
    if !src_dir.join("CMakeLists.txt").is_file() {
        return None;
    }
    let _ = std::fs::create_dir_all(&build);
    let cfg = std::process::Command::new("cmake")
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(&build)
        .output();
    if !matches!(&cfg, Ok(o) if o.status.success()) {
        return None;
    }
    let bld = std::process::Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .output();
    if !matches!(&bld, Ok(o) if o.status.success()) {
        return None;
    }
    so.is_file().then_some(so)
}

fn c_so_path() -> PathBuf {
    let src_dir = manifest_dir().join("c_src");
    let base = src_dir.join("build");
    if let Some(p) = ensure_cmake_built(&src_dir, "libtranslated_rust.so") {
        return p;
    }
    // Fall back to any single .so produced by the CMake build.
    if let Ok(rd) = std::fs::read_dir(&base) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                return p;
            }
        }
    }
    panic!(
        "C shared library not found under {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        base.display()
    );
}

fn newest(a: Option<PathBuf>, b: Option<PathBuf>) -> Option<PathBuf> {
    fn mtime(p: &Path) -> std::time::SystemTime {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH)
    }
    match (a, b) {
        (Some(x), Some(y)) => Some(if mtime(&x) >= mtime(&y) { x } else { y }),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

/// The feature flags this test binary was compiled with, as `cargo build` args.
///
/// The test binary and the cdylib it loads MUST agree on features, otherwise the
/// suite would silently verify the wrong configuration.
fn feature_args() -> Vec<&'static str> {
    let mut v = vec!["--no-default-features"];
    if cfg!(feature = "shadow_probe") {
        v.push("--features");
        v.push("shadow_probe");
    }
    v
}

/// Build the cdylib for THIS test binary's feature set into a dedicated target
/// directory, and return its path.
///
/// This exists because `cargo test` does not rebuild the `cdylib` artifact:
/// integration tests cannot link a cdylib, so cargo only builds the lib as an
/// rlib for them. Whatever sits in `target/debug/libjumpnode_lib.so` is left
/// over from the last `cargo build` and may have been built with a *different*
/// feature set — which would make the suite verify the wrong configuration, or
/// pass vacuously. Building it here makes any bare
/// `cargo test --no-default-features --features <combo>` correct by construction.
///
/// A separate `--target-dir` is used so this nested invocation cannot contend
/// with the outer `cargo test` for the build lock.
fn build_cdylib_for_this_config() -> Option<PathBuf> {
    let manifest = manifest_dir();
    let suffix = if cfg!(feature = "shadow_probe") {
        "xdiff-so-probe"
    } else {
        "xdiff-so-default"
    };
    let target_dir = manifest.join("target").join(suffix);

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let out = std::process::Command::new(cargo)
        .arg("build")
        .arg("--lib")
        .arg("--manifest-path")
        .arg(manifest.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir)
        .args(feature_args())
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .output()
        .ok()?;

    let so = target_dir.join("debug").join("libjumpnode_lib.so");
    if out.status.success() && so.is_file() {
        Some(so)
    } else {
        eprintln!(
            "note: nested `cargo build {}` did not produce {}:\n{}",
            feature_args().join(" "),
            so.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        None
    }
}

fn rust_so_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        // Preferred: a cdylib built right now, with exactly this feature set.
        if let Some(p) = build_cdylib_for_this_config() {
            return p;
        }

        // Fallback: whatever a previous `cargo build` left behind. Verified
        // below to match this binary's feature set, so a stale artifact fails
        // loudly instead of silently testing the wrong configuration.
        let target_root = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| manifest_dir().join("target"));
        let mut found: Option<PathBuf> = None;
        for profile in ["debug", "release"] {
            let p = target_root.join(profile).join("libjumpnode_lib.so");
            if p.is_file() {
                found = newest(found, Some(p));
            }
        }
        found.unwrap_or_else(|| {
            panic!(
                "Rust cdylib libjumpnode_lib.so not found under {} and the nested \
                 `cargo build` failed. Build it with `cargo build {}`.",
                target_root.display(),
                feature_args().join(" ")
            )
        })
    })
    .clone()
}

/// Guard against a feature mismatch between this test binary and the `.so` it
/// loaded: `probe_init` must be exported iff `shadow_probe` is enabled.
fn assert_so_matches_features(lib: &Library, path: &Path) {
    let has_probe =
        unsafe { lib.get::<unsafe extern "C" fn() -> c_int>(b"probe_init\0") }.is_ok();
    let want_probe = cfg!(feature = "shadow_probe");
    assert_eq!(
        has_probe,
        want_probe,
        "feature mismatch: {} {} `probe_init`, but this test binary was built \
         {}`shadow_probe`. The loaded .so does not match the configuration under \
         test. Rebuild with `cargo build {}`.",
        path.display(),
        if has_probe { "exports" } else { "does not export" },
        if want_probe { "WITH " } else { "WITHOUT " },
        feature_args().join(" ")
    );
}

/// Path accessors for `symbol_parity.rs`, so every test target inspects and
/// loads the exact same artifacts.
pub fn c_so_path_for_tests() -> PathBuf {
    c_so_path()
}

pub fn rust_so_path_for_tests() -> PathBuf {
    rust_so_path()
}

/// A freshly `dlopen`ed pair of libraries.
pub struct Pair {
    c_lib: Library,
    rust_lib: Library,
}

impl Pair {
    pub fn open() -> Pair {
        preload_libm();
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen C {}: {e}", c_path.display()));
        let rust_lib = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen Rust {}: {e}", rust_path.display()));
        assert_so_matches_features(&rust_lib, &rust_path);
        Pair { c_lib, rust_lib }
    }

    pub fn c(&self) -> Symbol<'_, JumpnodeFn> {
        unsafe { self.c_lib.get(b"jumpnode\0") }.expect("C .so does not export `jumpnode`")
    }

    pub fn rust(&self) -> Symbol<'_, JumpnodeFn> {
        unsafe { self.rust_lib.get(b"jumpnode\0") }
            .expect("Rust .so does not export `jumpnode` (missing #[no_mangle] wrapper?)")
    }
}

/// Process-wide pair, so most tests do not pay for repeated `dlopen`.
pub fn pair() -> &'static Pair {
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(Pair::open)
}

/// Call both `.so`s and assert byte-identical `int` results.
#[track_caller]
pub fn assert_same(mode: i32, node_id: i32, depth: i32, flags: i32) -> i32 {
    let p = pair();
    let cf = p.c();
    let rf = p.rust();
    let c_val = unsafe { cf(mode, node_id, depth, flags) };
    let r_val = unsafe { rf(mode, node_id, depth, flags) };
    assert_eq!(
        c_val, r_val,
        "DIVERGENCE jumpnode(mode={mode}, node_id={node_id}, depth={depth}, flags={flags}): \
         C returned {c_val} (0x{c_val:08x}), Rust returned {r_val} (0x{r_val:08x})"
    );
    c_val
}

// ---------------------------------------------------------------------------
// Probe ("shadow") harness — cargo feature `shadow_probe`.
//
// Pairs the Rust cdylib (built with `shadow_probe`) against
// `shadow_c/build/libshadow_c.so`, which `#include`s the untouched
// `c_src/src/lib.c`. This reaches the `static` helpers and lets `jumpnode` be
// driven with populated node storage — code the public API cannot reach.
// ---------------------------------------------------------------------------
#[cfg(feature = "shadow_probe")]
pub mod shadow {
    use super::{ensure_cmake_built, preload_libm, rust_so_path};
    use libloading::Library;
    use std::path::PathBuf;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    pub struct ShadowPair {
        pub c: Library,
        pub rust: Library,
    }

    fn shadow_c_so() -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("shadow_c");
        ensure_cmake_built(&dir, "libshadow_c.so").unwrap_or_else(|| {
            panic!(
                "shadow C library missing at {}/build/libshadow_c.so and it could not be \
                 built automatically. Build it with:\n  \
                 cd shadow_c && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                dir.display()
            )
        })
    }

    pub fn shadow() -> &'static ShadowPair {
        static P: OnceLock<ShadowPair> = OnceLock::new();
        P.get_or_init(|| {
            preload_libm();
            let cp = shadow_c_so();
            let rp = rust_so_path();
            let c = unsafe { Library::new(&cp) }
                .unwrap_or_else(|e| panic!("dlopen shadow C {}: {e}", cp.display()));
            let rust = unsafe { Library::new(&rp) }
                .unwrap_or_else(|e| panic!("dlopen Rust {}: {e}", rp.display()));
            // Fail loudly if the cdylib was built without the feature.
            if unsafe { rust.get::<unsafe extern "C" fn() -> i32>(b"probe_init\0") }.is_err() {
                panic!(
                    "{} does not export `probe_init`; rebuild with \
                     `cargo build --features shadow_probe`",
                    rp.display()
                );
            }
            ShadowPair { c, rust }
        })
    }

    /// Both libraries hold `static` mutable node storage, so probe tests (which
    /// mutate it) must not run concurrently within the test binary.
    pub fn lock() -> MutexGuard<'static, ()> {
        static M: OnceLock<Mutex<()>> = OnceLock::new();
        match M.get_or_init(|| Mutex::new(())).lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Fetch the same symbol, with the same signature, from both libraries.
    #[macro_export]
    macro_rules! both_syms {
        ($p:expr, $name:literal, $t:ty) => {{
            let cs: libloading::Symbol<$t> =
                unsafe { $p.c.get(concat!($name, "\0").as_bytes()) }
                    .expect(concat!("shadow C .so missing `", $name, "`"));
            let rs: libloading::Symbol<$t> =
                unsafe { $p.rust.get(concat!($name, "\0").as_bytes()) }
                    .expect(concat!("Rust .so missing `", $name, "`"));
            (cs, rs)
        }};
    }
}

/// Deterministic PRNG (xorshift64*), fixed seed per call site for reproducibility.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    /// Uniform over the whole `i32` range.
    pub fn i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len() as u64) as usize]
    }
    /// A `f64` biased toward the magnitudes `safe_double_to_int` branches on:
    /// small values, the `i32` clamp boundaries, huge values, and specials.
    pub fn shaped_f64(&mut self) -> f64 {
        match self.below(10) {
            0 => self.pick(&F64_BOUNDARIES),
            1 => (self.i32() as f64) / 8.0,
            2 => self.i32() as f64,
            3 => (self.next_u64() as f64) / 3.0,
            4 => -(self.next_u64() as f64) / 3.0,
            5 => f64::from_bits(self.next_u64()), // may be NaN / subnormal / inf
            6 => (self.below(2001) as f64 - 1000.0) + 0.5,
            7 => (self.below(200) as f64) * 1e300,
            8 => 2147483647.0 + (self.below(5) as f64) - 2.0,
            _ => -2147483648.0 + (self.below(5) as f64) - 2.0,
        }
    }

    /// A value biased toward interesting magnitudes: small ints, decimal-width
    /// boundaries and full-range values (mixes shapes the C branches on).
    pub fn shaped_i32(&mut self) -> i32 {
        match self.below(6) {
            0 => (self.below(21) as i64 - 10) as i32,
            1 => self.pick(&DECIMAL_WIDTH_BOUNDARIES),
            2 => self.pick(&ARG_BOUNDARIES),
            3 => (self.next_u64() as u16) as i32,
            4 => -((self.next_u64() as u16) as i32),
            _ => self.i32(),
        }
    }
}

/// Every value where the `%d` decimal width of `sprintf` changes, plus the
/// extremes. Mode `0003`'s result depends only on these widths.
pub const DECIMAL_WIDTH_BOUNDARIES: [i32; 44] = [
    i32::MIN,
    i32::MIN + 1,
    -2147483647,
    -1000000001,
    -1000000000,
    -999999999,
    -100000001,
    -100000000,
    -99999999,
    -10000001,
    -10000000,
    -9999999,
    -1000001,
    -1000000,
    -999999,
    -100001,
    -100000,
    -99999,
    -10001,
    -10000,
    -9999,
    -1001,
    -1000,
    -999,
    -101,
    -100,
    -99,
    -11,
    -10,
    -9,
    -1,
    0,
    1,
    9,
    10,
    99,
    100,
    999,
    1000,
    99999,
    100000,
    999999999,
    1000000000,
    i32::MAX,
];

/// Generic argument boundaries used across every axis.
pub const ARG_BOUNDARIES: [i32; 16] = [
    i32::MIN,
    i32::MIN + 1,
    -128,
    -127,
    -17,
    -16,
    -1,
    0,
    1,
    2,
    3,
    4,
    16,
    17,
    127,
    i32::MAX,
];

/// The five dispatch classes of `switch (operation_mode)`.
pub const MODES: [i32; 5] = [1, 2, 3, 4, 0];

/// `f64` values straddling every branch in `safe_double_to_int`.
pub const F64_BOUNDARIES: [f64; 26] = [
    0.0,
    -0.0,
    0.5,
    -0.5,
    1.0,
    -1.0,
    0.9999999999999999,
    -0.9999999999999999,
    2147483646.0,
    2147483647.0,
    2147483647.5,
    2147483648.0,
    2147483649.0,
    -2147483647.0,
    -2147483648.0,
    -2147483648.5,
    -2147483649.0,
    -2147483650.0,
    4294967296.0,
    1e300,
    -1e300,
    f64::INFINITY,
    f64::NEG_INFINITY,
    f64::NAN,
    f64::MIN_POSITIVE,
    -f64::MIN_POSITIVE,
];
