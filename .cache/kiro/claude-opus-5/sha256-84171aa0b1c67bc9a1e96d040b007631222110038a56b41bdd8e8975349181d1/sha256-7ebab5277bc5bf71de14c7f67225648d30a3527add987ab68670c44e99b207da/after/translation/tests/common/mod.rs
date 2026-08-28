//! Shared harness: loads the C reference `.so` and the Rust `.so` and exposes
//! `encode_quant` from each through `libloading`, so the Rust side is exercised
//! exactly like an external C caller would (including the `#[no_mangle]`
//! export wrapper).

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};

pub type EncodeQuantFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int) -> c_int;

fn workspace_root() -> PathBuf {
    // translation/ -> parent working directory
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn find_so(dir: &Path, name_hint: Option<&str>) -> PathBuf {
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
        .filter(|p| match name_hint {
            Some(h) => p.file_name().unwrap().to_str().unwrap().contains(h),
            None => true,
        })
        .collect();
    candidates.sort();
    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no .so found in {}", dir.display()))
}

/// Path of the C reference shared library produced by CMake.
///
/// `HARVEST_C_SO` overrides it, which lets the same tests run against a C
/// library built with different optimisation settings (the C source relies on
/// signed-overflow wrapping, so it is worth checking more than one).
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    assert!(
        build.is_dir(),
        "c_src/build missing -- build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    find_so(&build, None)
}

/// Path of the Rust `cdylib`. Derived from the test binary's own location
/// (`target/<profile>/deps/<test>`), so it follows whatever profile and
/// feature set cargo used for this run.
///
/// The crate is `crate-type = ["cdylib"]` only, so `cargo test` does not build
/// the shared object as a side effect of building the test binaries; if it is
/// missing we build it here with the same profile.
pub fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>/deps/<bin>")
        .to_path_buf();

    let is_release = profile_dir.file_name().and_then(|s| s.to_str()) == Some("release");
    let expected = profile_dir.join("libencode_quant_lib.so");
    if !expected.exists() {
        build_cdylib(is_release);
    }
    if expected.exists() {
        return expected;
    }
    find_so(&profile_dir, Some("encode_quant_lib"))
}

/// Build the cdylib once per test process, forwarding the feature set this test
/// binary was compiled with.
fn build_cdylib(release: bool) {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = std::process::Command::new(cargo);
        cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
        cmd.arg("build").arg("--lib");
        if release {
            cmd.arg("--release");
        }
        cmd.arg("--no-default-features");
        let feats = enabled_features();
        if !feats.is_empty() {
            cmd.arg("--features").arg(feats.join(","));
        }
        // Avoid inheriting the parent cargo's jobserver / target-dir surprises.
        cmd.env_remove("CARGO_MAKEFLAGS");
        let status = cmd.status().expect("spawn cargo build --lib");
        assert!(status.success(), "cargo build --lib failed: {status}");
    });
}

/// Features this test binary was compiled with. `translation` currently
/// declares no `[features]`, so this is empty; the helper keeps the harness
/// correct if features are ever added.
fn enabled_features() -> Vec<&'static str> {
    Vec::new()
}


pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: EncodeQuantFn,
    pub rust: EncodeQuantFn,
}

impl Pair {
    pub fn load() -> Self {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("load C .so");
            let rust_lib = Library::new(rust_so_path()).expect("load Rust .so");
            let c: Symbol<EncodeQuantFn> =
                c_lib.get(b"encode_quant\0").expect("C encode_quant symbol");
            let rust: Symbol<EncodeQuantFn> = rust_lib
                .get(b"encode_quant\0")
                .expect("Rust encode_quant symbol");
            let c = *c;
            let rust = *rust;
            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }

    #[track_caller]
    pub fn check(&self, args: [c_int; 6]) {
        let [uni, step, pred, tgt, tgt2, lsbit] = args;
        let expected = unsafe { (self.c)(uni, step, pred, tgt, tgt2, lsbit) };
        let actual = unsafe { (self.rust)(uni, step, pred, tgt, tgt2, lsbit) };
        assert_eq!(
            expected, actual,
            "encode_quant(uni={uni}, step={step}, pred={pred}, tgt={tgt}, \
             tgt2={tgt2}, lsbit={lsbit}): C={expected} Rust={actual}"
        );
    }
}

/// Deterministic xorshift64* PRNG so failures are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
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
        self.next_u64() as u32 as i32
    }
    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}
