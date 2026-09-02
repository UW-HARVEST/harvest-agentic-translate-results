//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls `dataentry` only
//! through the exported symbol, never through the Rust crate directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub type DataEntryFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Pair {
    // Keep the libraries alive for the whole process; the raw function
    // pointers below borrow from them.
    _c_lib: Library,
    _rust_lib: Library,
    c: DataEntryFn,
    rust: DataEntryFn,
    pub c_so: PathBuf,
    pub rust_so: PathBuf,
}

impl Pair {
    #[inline]
    pub fn c(&self, mode: c_int, p1: c_int, p2: c_int, p3: c_int) -> c_int {
        unsafe { (self.c)(mode, p1, p2, p3) }
    }

    #[inline]
    pub fn rust(&self, mode: c_int, p1: c_int, p2: c_int, p3: c_int) -> c_int {
        unsafe { (self.rust)(mode, p1, p2, p3) }
    }

    /// Differential assertion: the two `.so`s must return the identical `int`.
    #[track_caller]
    pub fn assert_same(&self, mode: c_int, p1: c_int, p2: c_int, p3: c_int) -> c_int {
        let expected = self.c(mode, p1, p2, p3);
        let actual = self.rust(mode, p1, p2, p3);
        assert_eq!(
            expected, actual,
            "divergence for dataentry(mode={mode}, param1={p1}, param2={p2}, param3={p3}): \
             C returned {expected}, Rust returned {actual}"
        );
        expected
    }

    /// Same as `assert_same` but also pins the value the C code returns, so a
    /// row's *expected* error sentinel is checked, not merely "both agree".
    #[track_caller]
    pub fn assert_same_and_eq(&self, mode: c_int, p1: c_int, p2: c_int, p3: c_int, want: c_int) {
        let got = self.assert_same(mode, p1, p2, p3);
        assert_eq!(
            got, want,
            "dataentry(mode={mode}, param1={p1}, param2={p2}, param3={p3}): \
             expected {want} from the C ground truth, both returned {got}"
        );
    }
}

unsafe impl Sync for Pair {}
unsafe impl Send for Pair {}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(load)
}

fn find_so(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let mut best: Option<PathBuf> = None;
    for entry in std::fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("so") {
            continue;
        }
        let name = path.file_name()?.to_str()?.to_string();
        if name.starts_with(prefix) {
            best = Some(path);
        }
    }
    best
}

fn load() -> Pair {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workdir = manifest.parent().expect("crate has a parent dir").to_path_buf();

    // C shared object: built by c_src/CMakeLists.txt into c_src/build/.
    let c_build = workdir.join("c_src").join("build");
    let c_so = find_so(&c_build, "lib").unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_build.display()
        )
    });

    // Rust shared object.
    //
    // IMPORTANT: the crate's only lib crate-type is `cdylib`, which integration
    // tests cannot link against, so `cargo test` does NOT build (or rebuild)
    // the `.so` as a dependency of this test binary. Loading whatever happens
    // to be sitting in target/<profile>/ would silently test a stale library.
    // Build it explicitly here, into a private target dir so the nested cargo
    // invocation does not contend for the lock the outer `cargo test` holds.
    let profile = profile_from_current_exe();
    let so_target = manifest.join("target").join(format!("harness-so-{profile}"));
    build_cdylib(&manifest, &so_target, &profile);

    let profile_dir = so_target.join(&profile);
    let rust_so = find_so(&profile_dir, "libdataentry_lib")
        .unwrap_or_else(|| panic!("no libdataentry_lib*.so in {}", profile_dir.display()));

    let c_lib = unsafe { Library::new(&c_so) }
        .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_so.display()));
    let rust_lib = unsafe { Library::new(&rust_so) }
        .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_so.display()));

    let c: DataEntryFn = unsafe {
        let s: Symbol<DataEntryFn> = c_lib
            .get(b"dataentry\0")
            .expect("C .so must export `dataentry`");
        *s
    };
    let rust: DataEntryFn = unsafe {
        let s: Symbol<DataEntryFn> = rust_lib
            .get(b"dataentry\0")
            .expect("Rust .so must export `dataentry`");
        *s
    };

    Pair {
        _c_lib: c_lib,
        _rust_lib: rust_lib,
        c,
        rust,
        c_so,
        rust_so,
    }
}

/// `current_exe()` is `<target>/<profile>/deps/<test-bin>`; recover `<profile>`
/// so the loaded `.so` is built with the same optimization settings as the
/// tests exercising it.
fn profile_from_current_exe() -> String {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("debug")
        .to_string()
}

fn build_cdylib(manifest: &Path, target_dir: &Path, profile: &str) {
    let mut cmd = std::process::Command::new(std::env::var("CARGO").unwrap_or("cargo".into()));
    cmd.current_dir(manifest)
        .arg("build")
        .arg("--lib")
        .arg("--target-dir")
        .arg(target_dir)
        // Mirror the feature selection of the test build so every feature
        // combination is verified against a matching `.so`.
        .args(feature_args());
    if profile != "debug" {
        cmd.arg("--profile").arg(profile);
    }
    // Do not inherit the outer build's jobserver / target dir.
    cmd.env_remove("CARGO_TARGET_DIR");
    cmd.env_remove("CARGO_MAKEFLAGS");
    let out = cmd.output().expect("failed to spawn cargo to build the cdylib");
    assert!(
        out.status.success(),
        "building the cdylib failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Feature flags to forward to the nested `cargo build`, so the `.so` the
/// harness builds matches the feature combination under test.
///
/// The crate declares no `[features]` (see `Cargo.toml`), so `FEATURES` is
/// empty and there is exactly one configuration. Enumerating them here rather
/// than assuming keeps the harness honest if a feature is ever added.
const FEATURES: &[&str] = &[];

fn feature_args() -> Vec<String> {
    if FEATURES.is_empty() {
        return Vec::new();
    }
    let enabled: Vec<&str> = FEATURES
        .iter()
        .copied()
        .filter(|f| {
            let key = format!("CARGO_FEATURE_{}", f.to_uppercase().replace('-', "_"));
            std::env::var(key).is_ok()
        })
        .collect();
    let mut args = vec!["--no-default-features".to_string()];
    if !enabled.is_empty() {
        args.push("--features".to_string());
        args.push(enabled.join(","));
    }
    args
}


/// Deterministic splitmix64 PRNG so every property-style row is reproducible.
pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Full-range `i32`, biased towards nothing: every bit pattern reachable.
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    /// Uniform in `[lo, hi]` inclusive.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    /// A mix of small values, boundary values and full-range values, which is
    /// what actually shakes out value-dependent and overflow bugs.
    pub fn spicy_i32(&mut self) -> i32 {
        const EDGES: [i32; 14] = [
            i32::MIN,
            i32::MIN + 1,
            i32::MIN / 2,
            -100_000,
            -1000,
            -10,
            -1,
            0,
            1,
            10,
            1000,
            100_000,
            i32::MAX - 1,
            i32::MAX,
        ];
        match self.next_u64() % 4 {
            0 => EDGES[(self.next_u64() % EDGES.len() as u64) as usize],
            1 => self.range(-16, 16),
            2 => self.range(-100_000, 100_000),
            _ => self.i32(),
        }
    }
}

/// Boundary values worth crossing the FFI boundary for any `int` parameter.
pub const EDGE_INTS: [i32; 13] = [
    i32::MIN,
    i32::MIN + 1,
    -100_000,
    -1000,
    -11,
    -10,
    -4,
    -1,
    0,
    1,
    1000,
    i32::MAX - 1,
    i32::MAX,
];
