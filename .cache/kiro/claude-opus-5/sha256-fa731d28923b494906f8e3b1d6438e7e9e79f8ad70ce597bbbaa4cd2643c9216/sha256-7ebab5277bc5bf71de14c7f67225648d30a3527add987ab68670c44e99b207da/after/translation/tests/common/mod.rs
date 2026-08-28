//! Shared helpers: locate and load the C and Rust shared libraries.
//!
//! Both libraries are loaded through `libloading` and every call goes through
//! the exported `extern "C"` symbol, so the Rust `#[no_mangle]` wrappers are
//! exercised exactly as an external caller would exercise them.
//!
//! Not every test binary uses every helper, hence the blanket `dead_code`
//! allowance.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// Workspace root (the directory holding `c_src/` and `translation/`).
pub fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_so(dir: &Path, prefer: Option<&str>) -> PathBuf {
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "so").unwrap_or(false))
        .collect();
    candidates.sort();

    if let Some(name) = prefer {
        if let Some(hit) = candidates.iter().find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains(name))
                .unwrap_or(false)
        }) {
            return hit.clone();
        }
    }

    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no .so found in {}", dir.display()))
}

/// Path to the C shared library built from `c_src/`.
///
/// The CMake project names the library after the parent directory, so the
/// exact file name is not known ahead of time and is globbed instead.
/// `HDR_BITRATE_C_SO` overrides the lookup (useful for checking the
/// translation against C built with different optimisation settings).
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HDR_BITRATE_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "HDR_BITRATE_C_SO={} not found", p.display());
        return p;
    }
    let build = workspace_root().join("c_src").join("build");
    assert!(
        build.is_dir(),
        "c_src/build does not exist - build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    find_so(&build, None)
}

/// Path to the Rust `cdylib`.
///
/// `cargo test` does not build a `cdylib`-only library target (it only needs
/// the crate metadata for the integration test), so the shared object may not
/// exist yet. It is looked for next to the test binary first, and otherwise
/// built on demand into a dedicated target directory. A dedicated directory is
/// used so the nested `cargo` invocation does not block on the build-directory
/// lock held by the outer `cargo test`.
///
/// `HDR_BITRATE_RUST_SO` overrides the whole lookup; `HDR_BITRATE_FEATURES`
/// (space or comma separated) and `HDR_BITRATE_NO_DEFAULT_FEATURES` are
/// forwarded to the nested build so the feature combination under test is
/// reproduced in the shared object.
pub fn rust_so_path() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(resolve_rust_so).clone()
}

fn resolve_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("HDR_BITRATE_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "HDR_BITRATE_RUST_SO={} not found", p.display());
        return p;
    }

    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("test binary must live in target/<profile>/deps");

    if let Some(p) = try_find_so(profile_dir, "hdr_bitrate_lib") {
        return p;
    }

    build_cdylib(profile_dir)
}

/// Build the `cdylib` with `cargo` into an isolated target directory.
fn build_cdylib(profile_dir: &Path) -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest.join("target").join("ffi-cdylib");

    // Mirror the profile the tests were compiled with.
    let profile = profile_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("debug")
        .to_string();

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(&manifest)
        .arg("build")
        .arg("--lib")
        .arg("--target-dir")
        .arg(&target_dir);
    if profile == "release" {
        cmd.arg("--release");
    }
    if std::env::var("HDR_BITRATE_NO_DEFAULT_FEATURES").is_ok() {
        cmd.arg("--no-default-features");
    }
    if let Ok(features) = std::env::var("HDR_BITRATE_FEATURES") {
        let features = features.replace(',', " ");
        let features = features.trim().to_string();
        if !features.is_empty() {
            cmd.arg("--features").arg(features);
        }
    }
    // Do not inherit the outer cargo's per-invocation environment.
    for var in [
        "CARGO_MAKEFLAGS",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_TARGET_DIR",
        "CARGO_BUILD_TARGET_DIR",
    ] {
        cmd.env_remove(var);
    }

    let out = cmd.output().expect("failed to spawn cargo build --lib");
    assert!(
        out.status.success(),
        "nested `cargo build --lib` failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let dir = target_dir.join(&profile);
    try_find_so(&dir, "hdr_bitrate_lib")
        .unwrap_or_else(|| panic!("cdylib not produced in {}", dir.display()))
}

fn try_find_so(dir: &Path, name: &str) -> Option<PathBuf> {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|e| e == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.contains(name))
                    .unwrap_or(false)
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// The signature of the single public entry point: `unsigned hdr_bitrate(const uint8_t *h)`.
pub type HdrBitrateFn = unsafe extern "C" fn(*const u8) -> std::ffi::c_uint;

/// A loaded implementation plus its resolved `hdr_bitrate` symbol.
pub struct Impl {
    _lib: Library,
    hdr_bitrate: HdrBitrateFn,
    pub name: &'static str,
}

impl Impl {
    fn load(path: &Path, name: &'static str) -> Self {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
        let sym: Symbol<HdrBitrateFn> = unsafe { lib.get(b"hdr_bitrate\0") }
            .unwrap_or_else(|e| panic!("{} does not export hdr_bitrate: {e}", path.display()));
        let hdr_bitrate = *sym;
        Impl {
            _lib: lib,
            hdr_bitrate,
            name,
        }
    }

    /// Call the exported `hdr_bitrate` on a header buffer.
    ///
    /// # Safety
    /// `header` must be at least 3 bytes long, matching the C contract.
    pub fn hdr_bitrate(&self, header: &[u8]) -> u32 {
        assert!(header.len() >= 3, "header must be >= 3 bytes");
        unsafe { (self.hdr_bitrate)(header.as_ptr()) }
    }
}

/// Load both implementations, C first.
pub fn load_both() -> (Impl, Impl) {
    let c = Impl::load(&c_so_path(), "C");
    let rust = Impl::load(&rust_so_path(), "Rust");
    (c, rust)
}

/// True when the C indexing expression stays inside the 90-byte `halfrate`
/// table for this header, i.e. when the C read is well defined.
///
/// ```text
/// plane = !!(h[1] & 0x8)          -> 0 | 1
/// row   = ((h[1] >> 1) & 3) - 1   -> -1 | 0 | 1 | 2
/// col   = h[2] >> 4               -> 0 ..= 15
/// flat  = plane * 45 + row * 15 + col
/// ```
pub fn flat_offset(h1: u8, h2: u8) -> i32 {
    let plane = i32::from(h1 & 0x8 != 0);
    let row = ((i32::from(h1) >> 1) & 3) - 1;
    let col = i32::from(h2) >> 4;
    plane * 45 + row * 15 + col
}

/// `true` if the flat offset lands inside the real 90-byte table.
pub fn is_defined(h1: u8, h2: u8) -> bool {
    let off = flat_offset(h1, h2);
    (0..90).contains(&off)
}
