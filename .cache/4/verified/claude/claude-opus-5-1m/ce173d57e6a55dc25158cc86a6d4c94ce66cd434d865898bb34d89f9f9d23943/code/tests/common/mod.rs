//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as *shared libraries* through `libloading`
//! and are only ever called through their exported `extern "C"` symbols, so the
//! `#[no_mangle]` export wrappers of the Rust crate are exercised exactly the
//! way an external C consumer would exercise them.

#![allow(dead_code)]

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

pub mod child;

/// Signature of the single public entry point: `float pow43(int x);`
pub type Pow43Fn = unsafe extern "C" fn(c_int) -> f32;

pub struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    pub c_pow43: Pow43Fn,
    pub rust_pow43: Pow43Fn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

// The two `Library` handles are only used to keep the dlopen()ed images alive;
// the raw function pointers we hand out are plain C functions with no interior
// state, so sharing them between threads is fine.
unsafe impl Send for Impls {}
unsafe impl Sync for Impls {}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Features of this crate that are enabled for the current test build.
///
/// One line per feature declared in `Cargo.toml [features]`, so that the
/// `cdylib` under test is always built with exactly the same configuration as
/// the test binary. The crate currently declares only the empty `default`
/// feature (see `CONFIGS.md` §0), hence the list is empty.
fn enabled_features() -> Vec<&'static str> {
    #[allow(unused_mut)]
    let mut f: Vec<&'static str> = Vec::new();
    // Example for future features:
    // if cfg!(feature = "foo") { f.push("foo"); }
    f
}

/// Builds the `cdylib` under test and returns its path.
///
/// **Why this is not just "look in `target/<profile>`":** for a crate whose only
/// library target is a `cdylib`, `cargo test` does *not* rebuild that `.so`
/// (the integration tests do not link against it), so an artifact left over
/// from an earlier `cargo build` would be loaded instead of the current source.
/// That would silently validate stale code, so the harness builds the `cdylib`
/// itself — into a private target directory, which also avoids fighting over
/// the enclosing `cargo test`'s lock — and then asserts that the artifact is
/// newer than every source file.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_POW43_SO") {
        return PathBuf::from(p);
    }
    let manifest = manifest_dir();
    let target_dir = manifest.join("target").join("test-cdylib");
    let profile_dir = if cfg!(debug_assertions) { "debug" } else { "release" };

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut args: Vec<String> = vec![
        "build".into(),
        "--lib".into(),
        "--no-default-features".into(),
        "--target-dir".into(),
        target_dir.display().to_string(),
    ];
    if !cfg!(debug_assertions) {
        args.push("--release".into());
    }
    let features = enabled_features();
    if !features.is_empty() {
        args.push("--features".into());
        args.push(features.join(","));
    }

    let mut attempts: Vec<Vec<String>> = Vec::new();
    let mut offline = args.clone();
    offline.push("--offline".into());
    attempts.push(offline);
    attempts.push(args);

    let mut last_err = String::new();
    let mut built = false;
    for a in &attempts {
        match Command::new(&cargo).current_dir(&manifest).args(a).output() {
            Ok(out) if out.status.success() => {
                built = true;
                break;
            }
            Ok(out) => {
                last_err = format!(
                    "cargo {}\n{}\n{}",
                    a.join(" "),
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            Err(e) => last_err = format!("cargo {}: {e}", a.join(" ")),
        }
    }

    let candidates = |dir: PathBuf| -> Option<PathBuf> {
        ["libpow43_lib.so", "libpow43_lib.dylib", "pow43_lib.dll"]
            .iter()
            .map(|n| dir.join(n))
            .find(|p| p.exists())
    };

    let so = if built {
        candidates(target_dir.join(profile_dir)).unwrap_or_else(|| {
            panic!("cdylib missing in {}", target_dir.join(profile_dir).display())
        })
    } else {
        // Fall back to an artifact produced by the enclosing build (next to the
        // test binary). The freshness assertion below still guarantees we never
        // silently test stale code.
        let sibling = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(Path::to_path_buf))
            .map(|deps| {
                if deps.file_name().map(|n| n == "deps").unwrap_or(false) {
                    deps.parent().unwrap_or(&deps).to_path_buf()
                } else {
                    deps
                }
            })
            .and_then(candidates);
        sibling.unwrap_or_else(|| {
            panic!("could not build or locate the cdylib under test:\n{last_err}")
        })
    };

    assert_fresher_than_sources(&so);
    so
}

/// Guards against ever testing a stale artifact.
fn assert_fresher_than_sources(so: &Path) {
    let so_time = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("cdylib mtime");
    let src = manifest_dir().join("src");
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                    if newest.as_ref().map(|(_, n)| t > *n).unwrap_or(true) {
                        newest = Some((p, t));
                    }
                }
            }
        }
    }
    if let Some((path, t)) = newest {
        assert!(
            so_time >= t,
            "the cdylib under test ({}) is OLDER than {} — the tests would \
             validate stale code",
            so.display(),
            path.display()
        );
    }
}

/// Builds `c_src/` with CMake (once) and returns the path of the C `.so`.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_POW43_SO") {
        return PathBuf::from(p);
    }
    let c_src = manifest_dir().join("c_src");
    let build = c_src.join("build");
    let existing = |build: &Path| -> Option<PathBuf> {
        for name in [
            "libtranslated_rust.so",
            "libtranslated_rust.dylib",
            "translated_rust.dll",
        ] {
            let cand = build.join(name);
            if cand.exists() {
                return Some(cand);
            }
        }
        None
    };
    if let Some(p) = existing(&build) {
        return p;
    }

    std::fs::create_dir_all(&build).expect("create c_src/build");
    let cfg = Command::new("cmake")
        .current_dir(&build)
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .output()
        .expect("run cmake (is cmake installed?)");
    assert!(
        cfg.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&cfg.stdout),
        String::from_utf8_lossy(&cfg.stderr)
    );
    let bld = Command::new("cmake")
        .current_dir(&build)
        .arg("--build")
        .arg(".")
        .output()
        .expect("run cmake --build");
    assert!(
        bld.status.success(),
        "cmake build failed:\n{}\n{}",
        String::from_utf8_lossy(&bld.stdout),
        String::from_utf8_lossy(&bld.stderr)
    );
    existing(&build).expect("C shared library missing after cmake build")
}

static IMPLS: OnceLock<Impls> = OnceLock::new();

/// Loads both shared libraries (once per test binary) and resolves `pow43`.
pub fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
            let c_sym: Symbol<Pow43Fn> = c_lib
                .get(b"pow43\0")
                .expect("symbol `pow43` missing from the C shared library");
            let rust_sym: Symbol<Pow43Fn> = rust_lib
                .get(b"pow43\0")
                .expect("symbol `pow43` missing from the Rust shared library");
            let c_pow43 = *c_sym;
            let rust_pow43 = *rust_sym;
            Impls {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_pow43,
                rust_pow43,
                c_path,
                rust_path,
            }
        }
    })
}

/// Calls `pow43(x)` in both libraries and returns `(c_result, rust_result)`.
pub fn call_both(x: c_int) -> (f32, f32) {
    let i = impls();
    unsafe { ((i.c_pow43)(x), (i.rust_pow43)(x)) }
}

/// Bit-exact comparison (so that `-0.0` vs `0.0` and distinct NaN payloads are
/// reported as differences, i.e. "byte-identical" in the strictest sense).
pub fn assert_bit_identical(x: c_int, ctx: &str) {
    let (c, r) = call_both(x);
    assert_eq!(
        c.to_bits(),
        r.to_bits(),
        "pow43({x}) mismatch [{ctx}]: C = {c:?} (0x{:08x}) vs Rust = {r:?} (0x{:08x})",
        c.to_bits(),
        r.to_bits()
    );
}

/// Asserts bit-identical results over an explicit list of inputs.
pub fn assert_all(inputs: impl IntoIterator<Item = c_int>, ctx: &str) {
    for x in inputs {
        assert_bit_identical(x, ctx);
    }
}

/// Deterministic SplitMix64 PRNG: property-style testing with a fixed seed so
/// every run examines exactly the same inputs.
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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform value in `[lo, hi]` (inclusive), for `lo <= hi`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }

    pub fn range_i32(&mut self, lo: c_int, hi: c_int) -> c_int {
        self.range(lo as i64, hi as i64) as c_int
    }
}

/// Inclusive input domain for which the C code's table index stays inside
/// `g_pow43[0 ..= 144]`, i.e. the range where the C behaviour is well defined.
///
/// * `x < 129`          -> index `16 + x`            -> needs `x >= -16`
/// * `129 <= x < 1024`  -> index `16 + ((8x+s)>>6)`  -> always in range
/// * `x >= 1024`        -> index `16 + ((x+s)>>6)`   -> needs `x <= 8223`
///   (`s = (x & 32) << 1`; the largest in-range `x` is 8223 because 8224..8255
///   have bit 5 set and therefore land on index 145).
pub const DOMAIN_MIN: c_int = -16;
pub const DOMAIN_MAX: c_int = 8223;

/// Recomputes the C table index for `x` exactly as `lib.c` does, used by the
/// tests to reason about which inputs stay inside the table.
pub fn c_table_index(x: c_int) -> c_int {
    if x < 129 {
        return 16 + x;
    }
    let mut x = x;
    if x < 1024 {
        x = x.wrapping_shl(3);
    }
    let sign = x.wrapping_mul(2) & 64;
    16i32.wrapping_add(x.wrapping_add(sign) >> 6)
}

/// True when `x` only reads defined table entries (`0 ..= 144`).
pub fn in_domain(x: c_int) -> bool {
    let idx = c_table_index(x);
    (0..=144).contains(&idx)
}

/// Which of the three branches of `pow43` the input takes.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Branch {
    /// `x < 129`: direct table read.
    A,
    /// `129 <= x < 1024`: `mult = 16`, `x <<= 3`.
    B,
    /// `x >= 1024`: `mult = 256`.
    C,
}

pub fn c_branch(x: c_int) -> Branch {
    if x < 129 {
        Branch::A
    } else if x < 1024 {
        Branch::B
    } else {
        Branch::C
    }
}

/// The value of `x` inside `pow43` after the optional `x <<= 3`.
pub fn c_shifted(x: c_int) -> c_int {
    if x < 129 {
        x
    } else if x < 1024 {
        x.wrapping_shl(3)
    } else {
        x
    }
}

/// `sign = 2 * x & 64` (after the optional shift); `0` or `64`.
pub fn c_sign(x: c_int) -> c_int {
    c_shifted(x).wrapping_mul(2) & 64
}

/// `mult`: `16` on branch B, `256` otherwise.
pub fn c_mult(x: c_int) -> c_int {
    if (129..1024).contains(&x) { 16 } else { 256 }
}

/// Numerator and denominator of `frac` as the C computes them (integers).
pub fn c_frac_parts(x: c_int) -> (c_int, c_int) {
    let xs = c_shifted(x);
    let sign = c_sign(x);
    ((xs & 63).wrapping_sub(sign), (xs & !63).wrapping_add(sign))
}

/// Collects `n` random inputs from `[lo, hi]` that satisfy `pred`.
/// Panics if the predicate is unsatisfiable in a reasonable number of tries
/// (that would mean the corresponding `CONFIGS.md` row is empty).
pub fn sample_where(
    rng: &mut Rng,
    lo: c_int,
    hi: c_int,
    n: usize,
    pred: impl Fn(c_int) -> bool,
) -> Vec<c_int> {
    let mut out = Vec::with_capacity(n);
    let mut tries = 0usize;
    while out.len() < n {
        tries += 1;
        assert!(
            tries < 100_000 + n * 100,
            "predicate unsatisfiable in [{lo}, {hi}] (empty CONFIGS row?)"
        );
        let x = rng.range_i32(lo, hi);
        if pred(x) {
            out.push(x);
        }
    }
    out
}

/// Loads a *second*, independent pair of handles (used to prove that neither
/// library keeps load-time state that changes results).
pub fn load_fresh() -> (Library, Library, Pow43Fn, Pow43Fn) {
    let i = impls();
    unsafe {
        let c_lib = Library::new(&i.c_path).expect("re-dlopen C library");
        let r_lib = Library::new(&i.rust_path).expect("re-dlopen Rust library");
        let c_sym: Symbol<Pow43Fn> = c_lib.get(b"pow43\0").expect("pow43 (C)");
        let r_sym: Symbol<Pow43Fn> = r_lib.get(b"pow43\0").expect("pow43 (Rust)");
        let c = *c_sym;
        let r = *r_sym;
        (c_lib, r_lib, c, r)
    }
}
