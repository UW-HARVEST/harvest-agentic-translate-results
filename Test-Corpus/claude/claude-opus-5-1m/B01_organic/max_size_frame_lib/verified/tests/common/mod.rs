//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries via `libloading` and resolves `max_size_frame`
//! from each. The Rust implementation is **never** called directly as a Rust
//! function — it is always reached through `dlopen`/`dlsym` on the compiled
//! `cdylib`, exactly as an external C consumer would, so the
//! `#[unsafe(no_mangle)] extern "C"` export wrapper is under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// ABI of the function under test: `tflac_u32 (tflac_u32, tflac_u32, tflac_u32)`.
pub type MaxSizeFrameFn = unsafe extern "C" fn(u32, u32, u32) -> u32;

pub struct Libs {
    /// `max_size_frame` from the C shared library (ground truth).
    c_fn: MaxSizeFrameFn,
    /// `max_size_frame` from the Rust shared library (under test).
    rust_fn: MaxSizeFrameFn,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
    // Keep the handles alive for the whole process; the raw fn pointers above
    // borrow from these.
    _c_lib: Library,
    _rust_lib: Library,
}

impl Libs {
    /// Call the C implementation through the FFI boundary.
    #[inline]
    pub fn c(&self, blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
        unsafe { (self.c_fn)(blocksize, channels, bitdepth) }
    }

    /// Call the Rust implementation through the FFI boundary.
    #[inline]
    pub fn rust(&self, blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
        unsafe { (self.rust_fn)(blocksize, channels, bitdepth) }
    }
}

/// Process-wide singleton so the libraries are `dlopen`ed once.
pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(load_libs)
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The exact file name CMake produces for the C library.
///
/// `c_src/CMakeLists.txt` derives the project name from the *parent* directory
/// of `c_src` (`cmake_path(GET ... PARENT_PATH parent)` then `FILENAME`), i.e.
/// the crate root's directory name. Pinning the name this way means a stray
/// `.so` dropped into `c_src/build` can never be silently adopted as the ground
/// truth (which an alphabetical "first match" would allow).
fn c_so_file_name() -> String {
    let project = manifest_dir()
        .file_name()
        .and_then(|n| n.to_str())
        .expect("crate root directory name")
        .to_string();
    format!("{DLL_PREFIX}{project}{DLL_SUFFIX}")
}

/// Locate the C `.so`, building it with CMake if it is missing **or stale**.
fn c_library_path() -> PathBuf {
    let c_src = manifest_dir().join("c_src");
    let build = c_src.join("build");
    let pinned = build.join(c_so_file_name());

    // Rebuild when absent OR older than any C source: without this check a
    // pre-existing .so silently becomes the "ground truth" and an edit to
    // c_src/src/lib.c would never be reflected, so the whole suite could pass
    // vacuously against an out-of-date reference.
    if pinned.is_file() && c_so_is_fresh(&pinned, &c_src) {
        return pinned;
    }

    // Not built yet (or stale): run the documented CMake build. Nothing inside
    // c_src's *sources* is modified; only the build/ directory is created.
    std::fs::create_dir_all(&build).expect("create c_src/build");
    let conf = Command::new("cmake")
        .current_dir(&build)
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .output()
        .expect("failed to spawn cmake (is cmake installed?)");
    assert!(
        conf.status.success(),
        "cmake configure failed:\n{}\n{}",
        String::from_utf8_lossy(&conf.stdout),
        String::from_utf8_lossy(&conf.stderr)
    );
    let built = Command::new("cmake")
        .current_dir(&build)
        .args(["--build", "."])
        .output()
        .expect("failed to spawn cmake --build");
    assert!(
        built.status.success(),
        "cmake --build failed:\n{}\n{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );

    assert!(
        pinned.is_file(),
        "cmake did not produce the expected C library {}. Files present: {:?}",
        pinned.display(),
        list_so(&build)
    );
    assert!(
        c_so_is_fresh(&pinned, &c_src),
        "C library {} is still older than a c_src source after rebuilding",
        pinned.display()
    );
    pinned
}

/// True when the built C library is at least as new as every `.c`/`.h` file and
/// `CMakeLists.txt` under `c_src`.
fn c_so_is_fresh(so_path: &Path, c_src: &Path) -> bool {
    let Ok(so_mtime) = std::fs::metadata(so_path).and_then(|m| m.modified()) else {
        return false;
    };
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    collect_newest_c(c_src, &mut newest);
    match newest {
        Some((_, src_mtime)) => so_mtime >= src_mtime,
        None => true,
    }
}

fn collect_newest_c(dir: &Path, newest: &mut Option<(PathBuf, std::time::SystemTime)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.filter_map(|e| e.ok()) {
        let p = e.path();
        // Skip the build output directory itself.
        if p.is_dir() {
            if p.file_name().and_then(|n| n.to_str()) != Some("build") {
                collect_newest_c(&p, newest);
            }
            continue;
        }
        let is_source = matches!(
            p.extension().and_then(|x| x.to_str()),
            Some("c") | Some("h")
        ) || p.file_name().and_then(|n| n.to_str()) == Some("CMakeLists.txt");
        if is_source {
            if let Ok(m) = std::fs::metadata(&p).and_then(|m| m.modified()) {
                if newest.as_ref().map(|(_, t)| m > *t).unwrap_or(true) {
                    *newest = Some((p, m));
                }
            }
        }
    }
}

/// All shared objects present in `dir`, for diagnostics.
fn list_so(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .filter(|n| n.ends_with(DLL_SUFFIX))
                .collect()
        })
        .unwrap_or_default()
}

/// Build (or rebuild) the Rust `cdylib` and return its path.
///
/// **Why this must build it explicitly:** the crate declares
/// `crate-type = ["cdylib"]` only, so the integration-test binaries do not link
/// against the library and `cargo test` therefore does **not** refresh
/// `target/<profile>/libmax_size_frame_lib.so`. Loading whatever `.so` happened
/// to be lying around silently tests a STALE binary — a mutation injected into
/// `src/lib.rs` would go undetected and every test would pass vacuously.
///
/// So we drive `cargo build --lib` ourselves into a dedicated target directory
/// (separate from the one the running `cargo test` owns, to avoid build-lock
/// contention) and then *assert* the artifact is newer than every Rust source
/// file. If the nested build cannot run, the freshness assertion still makes
/// staleness a loud failure instead of a silent pass.
fn rust_library_path() -> PathBuf {
    let so_name = format!("{DLL_PREFIX}max_size_frame_lib{DLL_SUFFIX}");
    let target_dir = manifest_dir().join("target").join("diff-so");
    let profile_dir = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let so_path = target_dir.join(profile_dir).join(&so_name);

    build_cdylib(&target_dir);

    assert!(
        so_path.is_file(),
        "nested `cargo build --lib` did not produce {}",
        so_path.display()
    );
    assert_fresher_than_sources(&so_path);
    so_path
}

/// Feature flags to forward to the nested build so the `.so` under test matches
/// the feature combination the current `cargo test` was invoked with. The
/// runner script sets `DIFF_TEST_FEATURE_ARGS` (e.g.
/// `--no-default-features --features foo,bar`).
fn feature_args() -> Vec<String> {
    match std::env::var("DIFF_TEST_FEATURE_ARGS") {
        Ok(s) => s.split_whitespace().map(str::to_string).collect(),
        Err(_) => Vec::new(),
    }
}

fn build_cdylib(target_dir: &Path) {
    static BUILT: OnceLock<()> = OnceLock::new();
    BUILT.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

        let run = |offline: bool| -> std::io::Result<std::process::Output> {
            let mut cmd = Command::new(&cargo);
            cmd.current_dir(manifest_dir())
                .arg("build")
                .arg("--lib")
                .arg("--target-dir")
                .arg(target_dir);
            if offline {
                cmd.arg("--offline");
            }
            if !cfg!(debug_assertions) {
                cmd.arg("--release");
            }
            cmd.args(feature_args());
            // Don't inherit the outer invocation's target dir / feature env.
            cmd.env_remove("CARGO_TARGET_DIR");
            cmd.env_remove("RUSTC_WORKSPACE_WRAPPER");
            cmd.output()
        };

        // Prefer --offline (this sandbox has no network); fall back if the local
        // registry cache is incomplete.
        let out = run(true).and_then(|o| if o.status.success() { Ok(o) } else { run(false) });

        let out = out.expect("failed to spawn `cargo build --lib` for the cdylib under test");
        assert!(
            out.status.success(),
            "nested `cargo build --lib` FAILED; the .so under test would be stale.\n\
             stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    });
}

/// Guard against silently testing a stale artifact.
fn assert_fresher_than_sources(so_path: &Path) {
    let so_mtime = std::fs::metadata(so_path)
        .and_then(|m| m.modified())
        .expect("stat .so");

    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    collect_newest_rs(&manifest_dir().join("src"), &mut newest);

    if let Some((src, src_mtime)) = newest {
        assert!(
            so_mtime >= src_mtime,
            "STALE ARTIFACT: {} is older than {}.\n\
             The tests would be validating an out-of-date library. Run `cargo build`.",
            so_path.display(),
            src.display()
        );
    }
}

fn collect_newest_rs(dir: &Path, newest: &mut Option<(PathBuf, std::time::SystemTime)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.filter_map(|e| e.ok()) {
        let p = e.path();
        if p.is_dir() {
            collect_newest_rs(&p, newest);
        } else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
            if let Ok(m) = std::fs::metadata(&p).and_then(|m| m.modified()) {
                if newest.as_ref().map(|(_, t)| m > *t).unwrap_or(true) {
                    *newest = Some((p, m));
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
const DLL_PREFIX: &str = "lib";
#[cfg(target_os = "linux")]
const DLL_SUFFIX: &str = ".so";
#[cfg(target_os = "macos")]
const DLL_PREFIX: &str = "lib";
#[cfg(target_os = "macos")]
const DLL_SUFFIX: &str = ".dylib";
#[cfg(target_os = "windows")]
const DLL_PREFIX: &str = "";
#[cfg(target_os = "windows")]
const DLL_SUFFIX: &str = ".dll";

fn load_libs() -> Libs {
    let c_path = c_library_path();
    let rust_path = rust_library_path();

    unsafe {
        let c_lib = Library::new(&c_path)
            .unwrap_or_else(|e| panic!("dlopen C lib {}: {e}", c_path.display()));
        let rust_lib = Library::new(&rust_path)
            .unwrap_or_else(|e| panic!("dlopen Rust lib {}: {e}", rust_path.display()));

        let c_sym: Symbol<MaxSizeFrameFn> = c_lib.get(b"max_size_frame\0").unwrap_or_else(|e| {
            panic!("C lib {} has no `max_size_frame`: {e}", c_path.display())
        });
        let rust_sym: Symbol<MaxSizeFrameFn> =
            rust_lib.get(b"max_size_frame\0").unwrap_or_else(|e| {
                panic!(
                    "Rust lib {} does not export `max_size_frame`: {e}",
                    rust_path.display()
                )
            });

        // Copy out the raw fn pointers (Copy + effectively 'static because the
        // Library handles are moved into the returned struct and never dropped).
        let c_fn: MaxSizeFrameFn = *c_sym;
        let rust_fn: MaxSizeFrameFn = *rust_sym;

        Libs {
            c_fn,
            rust_fn,
            c_path,
            rust_path,
            _c_lib: c_lib,
            _rust_lib: rust_lib,
        }
    }
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

/// Assert C and Rust agree bit-for-bit for one input triple.
#[inline]
#[track_caller]
pub fn assert_same(l: &Libs, blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
    let got_c = l.c(blocksize, channels, bitdepth);
    let got_rust = l.rust(blocksize, channels, bitdepth);
    assert_eq!(
        got_c.to_ne_bytes(),
        got_rust.to_ne_bytes(),
        "DIVERGENCE for max_size_frame(blocksize={blocksize}, channels={channels}, \
         bitdepth={bitdepth}) [hex: bs=0x{blocksize:08X} ch=0x{channels:08X} bd=0x{bitdepth:08X}]\n\
         \x20 C    = {got_c} (0x{got_c:08X})\n\
         \x20 Rust = {got_rust} (0x{got_rust:08X})"
    );
    got_c
}

/// Assert C and Rust agree *and* that the shared value equals `expected`.
/// Used by the error-surface rows that predict a concrete value.
#[inline]
#[track_caller]
pub fn assert_same_and_eq(l: &Libs, blocksize: u32, channels: u32, bitdepth: u32, expected: u32) {
    let got = assert_same(l, blocksize, channels, bitdepth);
    assert_eq!(
        got, expected,
        "both libraries agree ({got}) but the documented expectation for \
         max_size_frame({blocksize}, {channels}, {bitdepth}) is {expected}"
    );
}

/// Independent re-derivation of the C expression, used as a third opinion so a
/// shared C/Rust misreading cannot hide behind a pure `A == B` comparison.
///
/// Deliberately written by a DIFFERENT route from `src/lib.rs`: it computes in
/// `u64` and masks to 32 bits after every operation, rather than using `u32`
/// wrapping operators, and it collapses the three terms into the single derived
/// multiplier `M` that the algebra implies. If `src/lib.rs` mirrored the C's
/// operator tree incorrectly but self-consistently, this would still catch it.
///
/// Justification of the algebra (all arithmetic mod 2^32, `[p]` is 1 or 0):
/// ```text
/// T1 + T2 + T3 = bs*bd*(ch*[ch!=2]) + bs*bd*[ch==2] + bs*(bd + [bd!=32])*[ch==2]
///              = bs * ( bd*ch*[ch!=2] + bd*[ch==2] + (bd + [bd!=32])*[ch==2] )
///              = bs * M
/// f = 18 + ch + ((bs*M + 7) / 8)      (unsigned division)
/// ```
/// `uint32_t` is `unsigned int` on this platform (verified), so every C
/// subexpression is `unsigned int` and all overflow is defined wraparound —
/// there is no signed-overflow UB to model.
pub fn oracle(blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
    const M32: u64 = 0xFFFF_FFFF;

    let bs = blocksize as u64;
    let ch = channels as u64;
    let bd = bitdepth as u64;

    let eq2: u64 = if channels == 2 { 1 } else { 0 };
    let ne2: u64 = if channels != 2 { 1 } else { 0 };
    let ne32: u64 = if bitdepth != 32 { 1 } else { 0 };

    // M = bd*ch*[ch!=2] + bd*[ch==2] + (bd + [bd!=32])*[ch==2]
    let m = ((bd * ((ch * ne2) & M32)) & M32)
        .wrapping_add((bd * eq2) & M32)
        .wrapping_add((((bd + ne32) & M32) * eq2) & M32)
        & M32;

    let product = (bs * m) & M32;
    let numerator = (product + 7) & M32;
    let quotient = numerator / 8; // unsigned division, as in C

    (((18 + ch) & M32) + quotient) as u32 & M32 as u32
}

/// Assert C == Rust == independent oracle.
#[inline]
#[track_caller]
pub fn assert_same_triple(l: &Libs, blocksize: u32, channels: u32, bitdepth: u32) {
    let got = assert_same(l, blocksize, channels, bitdepth);
    let want = oracle(blocksize, channels, bitdepth);
    assert_eq!(
        got, want,
        "C and Rust agree ({got}) but the independent oracle says {want} for \
         max_size_frame({blocksize}, {channels}, {bitdepth})"
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — property-style testing with a fixed seed
// ---------------------------------------------------------------------------

pub struct Rng(u64);

/// Base seed; each test adds a distinct salt so rows explore different points.
pub const BASE_SEED: u64 = 0x5EED_C0FF_EE00_0000;

impl Rng {
    pub fn new(salt: u64) -> Self {
        Rng(BASE_SEED ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform over the whole 32-bit range.
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `lo..=hi`.
    #[inline]
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }

    /// Pick a random element of a slice.
    #[inline]
    pub fn pick(&mut self, xs: &[u32]) -> u32 {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

/// The interesting boundary values referenced by `CONFIGS.md` row 26 and by the
/// error-surface table.
pub const BOUNDARY_VALUES: &[u32] = &[
    0,
    1,
    2,
    3,
    4,
    7,
    8,
    31,
    32,
    33,
    64,
    4096,
    65535,
    65536,
    0x7FFF_FFFF,
    0x8000_0000,
    0xFFFF_FFFE,
    0xFFFF_FFFF,
];

/// Block sizes a real FLAC encoder emits.
pub const FLAC_BLOCKSIZES: &[u32] = &[
    192, 576, 1152, 2304, 4608, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65535,
];

/// Bit depths a real FLAC encoder emits.
pub const FLAC_BITDEPTHS: &[u32] = &[4, 8, 12, 16, 20, 24, 32];
