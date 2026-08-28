// Differential-test harness.
//
// Loads BOTH shared objects with `libloading` and calls `dataentry` only
// through its exported C symbol -- never through a direct Rust call -- so the
// `#[unsafe(no_mangle)] extern "C"` wrapper and the C ABI are under test too.
//
// Both `.so`s are (re)built from source on harness start-up so a stale artifact
// can never mask a source-level divergence.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub type DataEntryFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Repository root: the directory holding both `c_src/` and `translation/`.
pub fn repo_root() -> PathBuf {
    manifest_dir().parent().expect("manifest has a parent").to_path_buf()
}

fn first_so(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    found.sort();
    found.into_iter().next()
}

// ---------------------------------------------------------------------------
// Build the C shared library (out-of-tree: nothing under c_src/ is touched)
// ---------------------------------------------------------------------------

pub fn c_so_path() -> &'static PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        // Allows re-running the whole suite against a C `.so` built with a
        // different optimization level (the C source has signed-overflow UB,
        // so we verify the Rust matches gcc at -O0/-O1/-O2/-O3).
        if let Ok(p) = std::env::var("C_SO_OVERRIDE") {
            let p = PathBuf::from(p);
            assert!(p.is_file(), "C_SO_OVERRIDE={} not a file", p.display());
            return p;
        }

        let src = repo_root().join("c_src");
        assert!(src.join("CMakeLists.txt").is_file(), "missing {}", src.display());

        let build = manifest_dir().join("target").join("c_build");
        std::fs::create_dir_all(&build).expect("create c_build dir");

        // Exactly the configure line the task documents -- no CMAKE_BUILD_TYPE,
        // so gcc's default (unoptimized) flags are used, matching the reference.
        let cfg = Command::new("cmake")
            .arg("-S")
            .arg(&src)
            .arg("-B")
            .arg(&build)
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .output()
            .expect("run cmake configure");
        assert!(
            cfg.status.success(),
            "cmake configure failed:\n{}\n{}",
            String::from_utf8_lossy(&cfg.stdout),
            String::from_utf8_lossy(&cfg.stderr)
        );

        let bld = Command::new("cmake")
            .arg("--build")
            .arg(&build)
            .output()
            .expect("run cmake build");
        assert!(
            bld.status.success(),
            "cmake build failed:\n{}\n{}",
            String::from_utf8_lossy(&bld.stdout),
            String::from_utf8_lossy(&bld.stderr)
        );

        first_so(&build).unwrap_or_else(|| panic!("no .so produced in {}", build.display()))
    })
}

// ---------------------------------------------------------------------------
// Build the Rust cdylib (`cargo test` alone does not emit it)
// ---------------------------------------------------------------------------

pub fn rust_so_path() -> &'static PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        if let Ok(p) = std::env::var("RUST_SO_OVERRIDE") {
            let p = PathBuf::from(p);
            assert!(p.is_file(), "RUST_SO_OVERRIDE={} not a file", p.display());
            return p;
        }

        // A dedicated target dir keeps this out of the lock held by the
        // `cargo test` invocation that is running us.
        let target = manifest_dir().join("target").join("so_build");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

        let mut cmd = Command::new(cargo);
        cmd.current_dir(manifest_dir())
            .arg("build")
            .arg("--offline")
            .arg("--release")
            .arg("--lib")
            .arg("--target-dir")
            .arg(&target);

        // Propagate the feature selection of the running test binary so the
        // loaded `.so` is built in the *same* configuration. The crate declares
        // no `[features]`, so `CDYLIB_FEATURE_ARGS` is empty in every supported
        // combination; the hook exists so adding a feature later stays honest.
        if let Ok(extra) = std::env::var("CDYLIB_FEATURE_ARGS") {
            for a in extra.split_whitespace() {
                cmd.arg(a);
            }
        }

        // Cargo sets these for the child build; clearing avoids confusing it.
        for k in [
            "RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_BUILD_TARGET_DIR",
            "CARGO_TARGET_DIR",
        ] {
            cmd.env_remove(k);
        }

        let out = cmd.output().expect("run cargo build for cdylib");
        assert!(
            out.status.success(),
            "cargo build --lib failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );

        let dir = target.join("release");
        let p = dir.join("libdataentry_lib.so");
        if p.is_file() {
            return p;
        }
        first_so(&dir).unwrap_or_else(|| panic!("no .so produced in {}", dir.display()))
    })
}

// ---------------------------------------------------------------------------
// Loaded targets
// ---------------------------------------------------------------------------

pub struct Target {
    pub label: &'static str,
    pub path: PathBuf,
    pub dataentry: DataEntryFn,
    _lib: &'static Library,
}

fn load(label: &'static str, path: &Path) -> Target {
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
    // Leak so the library outlives every borrowed symbol for the whole run.
    let lib: &'static Library = Box::leak(Box::new(lib));
    let sym: Symbol<DataEntryFn> = unsafe { lib.get(b"dataentry\0") }
        .unwrap_or_else(|e| panic!("dlsym(dataentry) in {} failed: {e}", path.display()));
    Target { label, path: path.to_path_buf(), dataentry: *sym, _lib: lib }
}

pub struct Pair {
    pub c: Target,
    pub rust: Target,
}

pub fn pair() -> &'static Pair {
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: load("C", c_so_path()),
        rust: load("Rust", rust_so_path()),
    })
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

/// Call both `.so`s and return `(c_result, rust_result)`.
pub fn call_both(mode: i32, p1: i32, p2: i32, p3: i32) -> (i32, i32) {
    let p = pair();
    let c = unsafe { (p.c.dataentry)(mode, p1, p2, p3) };
    let r = unsafe { (p.rust.dataentry)(mode, p1, p2, p3) };
    (c, r)
}

/// Assert byte-identical results from both `.so`s.
#[track_caller]
pub fn same(mode: i32, p1: i32, p2: i32, p3: i32) -> i32 {
    let (c, r) = call_both(mode, p1, p2, p3);
    assert_eq!(
        c, r,
        "DIVERGENCE dataentry(mode={mode}, p1={p1}, p2={p2}, p3={p3}): C={c} Rust={r}"
    );
    // Byte-level equality of the 4-byte return value.
    assert_eq!(c.to_ne_bytes(), r.to_ne_bytes(), "return bytes differ for ({mode},{p1},{p2},{p3})");
    c
}

/// Assert both agree *and* that the shared value equals `expected`
/// (used for error rows where the exact sentinel matters).
#[track_caller]
pub fn same_eq(mode: i32, p1: i32, p2: i32, p3: i32, expected: i32) {
    let got = same(mode, p1, p2, p3);
    assert_eq!(
        got, expected,
        "dataentry(mode={mode}, p1={p1}, p2={p2}, p3={p3}) = {got}, expected sentinel {expected}"
    );
}

/// Assert both agree and the shared value is NOT `forbidden`.
#[track_caller]
pub fn same_ne(mode: i32, p1: i32, p2: i32, p3: i32, forbidden: i32) -> i32 {
    let got = same(mode, p1, p2, p3);
    assert_ne!(
        got, forbidden,
        "dataentry(mode={mode}, p1={p1}, p2={p2}, p3={p3}) unexpectedly hit sentinel {forbidden}"
    );
    got
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) -- fixed seeds for reproducibility
// ---------------------------------------------------------------------------

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

    /// Uniform over the whole `i32` range (exercises negatives + overflow).
    pub fn any_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    /// Uniform in `[lo, hi]` inclusive.
    pub fn in_range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    /// A value biased toward boundaries, with the rest uniform over `i32`.
    pub fn edgy_i32(&mut self) -> i32 {
        const EDGES: [i32; 14] = [
            0,
            1,
            -1,
            2,
            -2,
            3,
            4,
            5,
            10,
            11,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
        ];
        if self.next_u64() % 3 == 0 {
            EDGES[(self.next_u64() % EDGES.len() as u64) as usize]
        } else {
            self.any_i32()
        }
    }
}

// ---------------------------------------------------------------------------
// Model constants mirrored from the C source (for documenting expectations)
// ---------------------------------------------------------------------------

/// `sizeof(DataEntry)` == 4 + 4 + 32.
pub const SIZEOF_DATAENTRY: usize = 40;
/// `#define NAME_LENGTH 32`
pub const NAME_LENGTH: usize = 32;
/// `#define MAX_ENTRIES 10` -- dead in the C source, never enforced.
pub const MAX_ENTRIES: i32 = 10;
/// `static int lookup_table[4][3]`
pub const LOOKUP_TABLE: [[i32; 3]; 4] =
    [[10, 20, 30], [40, 50, 60], [70, 80, 90], [100, 110, 120]];

/// `param1` values large enough that `malloc(count * 40)` is guaranteed to fail:
/// each requests >= 40 GiB, comfortably above total RAM (no swap), so the
/// kernel's overcommit heuristic rejects it outright.
pub const ALLOC_FAIL_COUNTS: [i32; 4] =
    [i32::MAX, i32::MAX - 1, 1 << 30, (1 << 30) + 123_457];

/// Lower bound the `ALLOC_FAIL_COUNTS` requests must exceed, in bytes.
pub const ALLOC_FAIL_MIN_BYTES: i64 = 32 * (1i64 << 30);

/// Upper bound on `count` used by randomized tests so allocations stay cheap.
pub const SANE_COUNT_MAX: i32 = 4096;

/// Clamp an arbitrary `i32` into a count that keeps allocations cheap while
/// still covering the `param1 <= 0` default-count branch.
pub fn sane_param1(v: i32) -> i32 {
    if v <= 0 {
        // keep the "<= 0 => default count" branch reachable
        (v % 8).max(-7)
    } else {
        (v % SANE_COUNT_MAX).max(1)
    }
}
