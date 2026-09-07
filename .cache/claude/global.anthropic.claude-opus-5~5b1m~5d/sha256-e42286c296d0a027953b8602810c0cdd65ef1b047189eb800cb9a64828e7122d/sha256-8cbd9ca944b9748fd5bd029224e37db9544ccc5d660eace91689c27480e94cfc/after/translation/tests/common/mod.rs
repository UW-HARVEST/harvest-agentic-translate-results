//! Shared harness: loads BOTH the C `.so` and the Rust `.so` through
//! `libloading` and calls `dataentry` across the FFI boundary in both.
//!
//! The Rust implementation is *never* called directly — always via the
//! `cdylib`'s exported `dataentry` symbol, so the `#[no_mangle]`/`extern "C"`
//! wrapper is under test too.

// Each integration-test binary includes this module and uses a different subset
// of it, so per-binary "never used" warnings are expected and unhelpful.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

pub type DataEntryFn = unsafe extern "C" fn(
    ::std::os::raw::c_int,
    ::std::os::raw::c_int,
    ::std::os::raw::c_int,
    ::std::os::raw::c_int,
) -> ::std::os::raw::c_int;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_sources() -> Vec<PathBuf> {
    let root = repo_root();
    vec![
        root.join("c_src/src/lib.c"),
        root.join("c_src/include/lib.h"),
        root.join("c_src/CMakeLists.txt"),
    ]
}

fn rust_sources() -> Vec<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    vec![manifest.join("src/lib.rs"), manifest.join("Cargo.toml")]
}

/// `true` when `artifact` exists and is at least as new as every source file.
fn is_fresh(artifact: &Path, sources: &[PathBuf]) -> bool {
    let Ok(a) = std::fs::metadata(artifact).and_then(|m| m.modified()) else {
        return false;
    };
    sources.iter().all(|s| {
        match std::fs::metadata(s).and_then(|m| m.modified()) {
            Ok(t) => a >= t,
            Err(_) => true, // missing source: nothing to be stale against
        }
    })
}

/// Build (if needed) and locate the C shared library.
fn c_so_path() -> PathBuf {
    let root = repo_root();
    let c_src = root.join("c_src");
    let build = c_src.join("build");

    if let Some(p) = find_so(&build) {
        if is_fresh(&p, &c_sources()) {
            return p;
        }
    }

    std::fs::create_dir_all(&build).expect("mkdir c_src/build");
    let st = Command::new("cmake")
        .current_dir(&build)
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .status()
        .expect("run cmake");
    assert!(st.success(), "cmake configure failed");
    let st = Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .status()
        .expect("run cmake --build");
    assert!(st.success(), "cmake build failed");

    find_so(&build).expect("C .so not found after building c_src")
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut hits: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// Build (if needed) and locate the Rust cdylib.
///
/// IMPORTANT: `cargo test` does **not** rebuild the `cdylib` artifact (the test
/// targets only need the `rlib`), so the `.so` sitting in `target/<profile>/`
/// can easily be stale. A stale `.so` makes every differential assertion pass
/// vacuously, so we:
///   1. use cargo's own artifact only when it is newer than `src/lib.rs`;
///   2. otherwise rebuild the cdylib ourselves into a SEPARATE target dir
///      (`target/harness`) so we never fight cargo's build lock;
///   3. and finally re-assert freshness in `Pair::load` (see `assert_fresh`).
fn rust_so_path() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let srcs = rust_sources();
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };

    // 1. cargo's artifact for the profile this test binary was built under.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|deps| deps.parent()) {
            let p = profile_dir.join("libdataentry_lib.so");
            if is_fresh(&p, &srcs) {
                return p;
            }
        }
    }

    // 1b. a previously built harness copy.
    let harness_dir = manifest.join("target/harness");
    let harness_so = harness_dir.join(profile).join("libdataentry_lib.so");
    if is_fresh(&harness_so, &srcs) {
        return harness_so;
    }

    // 2. rebuild it ourselves, out of the way of cargo's main target dir.
    let mut cmd = Command::new(env!("CARGO"));
    cmd.current_dir(manifest)
        .args(["build", "--offline", "--lib"])
        .arg("--target-dir")
        .arg(&harness_dir);
    if !cfg!(debug_assertions) {
        cmd.arg("--release");
    }
    // Propagate the feature selection this test binary was compiled with, so
    // the loaded cdylib matches the configuration under test.
    if let Ok(feats) = std::env::var("HARNESS_CARGO_FEATURES") {
        if feats == "__nodefault__" {
            cmd.arg("--no-default-features");
        } else if !feats.is_empty() {
            cmd.arg("--no-default-features").arg("--features").arg(feats);
        }
    }
    let st = cmd.status().expect("run cargo build for the harness cdylib");
    assert!(st.success(), "harness `cargo build --lib` failed");

    assert!(
        harness_so.is_file(),
        "harness cdylib missing after build: {}",
        harness_so.display()
    );
    harness_so
}

/// Guard against the *silent vacuous pass*: if the `.so` we are about to dlopen
/// is older than the sources it is built from, every differential assertion
/// would compare against stale code and pass for the wrong reason.
fn assert_fresh(so: &Path, sources: &[PathBuf], what: &str) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", so.display()));
    for src in sources {
        if !src.is_file() {
            continue;
        }
        let src_mtime = std::fs::metadata(src)
            .and_then(|m| m.modified())
            .unwrap_or_else(|e| panic!("stat {}: {e}", src.display()));
        assert!(
            so_mtime >= src_mtime,
            "STALE {what}: {} is older than {}. The tests would have compared \
             against out-of-date code and passed vacuously. Rebuild first \
             (cargo build / cmake --build c_src/build).",
            so.display(),
            src.display()
        );
    }
}

pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c: DataEntryFn,
    rust: DataEntryFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Pair {
    pub fn load() -> Pair {
        // Resolve (and if necessary build) each .so exactly once per process,
        // so parallel tests do not race on the build.
        static C_SO: OnceLock<PathBuf> = OnceLock::new();
        static RUST_SO: OnceLock<PathBuf> = OnceLock::new();
        static BUILD_LOCK: Mutex<()> = Mutex::new(());

        let (c_path, rust_path) = {
            let _g = BUILD_LOCK.lock().unwrap();
            (
                C_SO.get_or_init(c_so_path).clone(),
                RUST_SO.get_or_init(rust_so_path).clone(),
            )
        };

        assert_ne!(
            c_path.canonicalize().unwrap(),
            rust_path.canonicalize().unwrap(),
            "the harness must load two DIFFERENT shared objects"
        );

        assert_fresh(&c_path, &c_sources(), "C .so");
        assert_fresh(&rust_path, &rust_sources(), "Rust .so");

        unsafe {
            let c_lib = Library::new(&c_path).expect("dlopen C .so");
            let rust_lib = Library::new(&rust_path).expect("dlopen Rust .so");

            let c_sym: Symbol<DataEntryFn> =
                c_lib.get(b"dataentry\0").expect("C exports `dataentry`");
            let rust_sym: Symbol<DataEntryFn> = rust_lib
                .get(b"dataentry\0")
                .expect("Rust .so exports `dataentry`");

            let c = *c_sym;
            let rust = *rust_sym;

            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
                c_path,
                rust_path,
            }
        }
    }

    #[inline]
    pub fn call_c(&self, m: i32, a: i32, b: i32, d: i32) -> i32 {
        unsafe { (self.c)(m, a, b, d) }
    }

    #[inline]
    pub fn call_rust(&self, m: i32, a: i32, b: i32, d: i32) -> i32 {
        unsafe { (self.rust)(m, a, b, d) }
    }

    /// Assert byte-identical (bit-identical `int`) results from both `.so`s.
    #[track_caller]
    pub fn assert_same(&self, m: i32, a: i32, b: i32, d: i32) -> i32 {
        let cv = self.call_c(m, a, b, d);
        let rv = self.call_rust(m, a, b, d);
        assert_eq!(
            cv, rv,
            "divergence for dataentry(mode={m}, p1={a}, p2={b}, p3={d}): \
             C returned {cv} (0x{cv:08x}), Rust returned {rv} (0x{rv:08x})"
        );
        // Also compare the raw little-endian bytes of the returned int.
        assert_eq!(
            cv.to_le_bytes(),
            rv.to_le_bytes(),
            "byte-level divergence for dataentry({m}, {a}, {b}, {d})"
        );
        cv
    }
}

/// Deterministic xorshift64* PRNG — fixed seed per row for reproducibility.
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
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Values biased toward interesting boundaries.
    pub fn spicy_i32(&mut self) -> i32 {
        const SPICE: [i32; 16] = [
            0,
            1,
            -1,
            2,
            -2,
            3,
            4,
            5,
            -5,
            10,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
            0x2000_0000,
            -0x2000_0000,
        ];
        let r = self.next_u64();
        if r % 3 == 0 {
            SPICE[(r >> 8) as usize % SPICE.len()]
        } else {
            self.next_i32()
        }
    }
}
