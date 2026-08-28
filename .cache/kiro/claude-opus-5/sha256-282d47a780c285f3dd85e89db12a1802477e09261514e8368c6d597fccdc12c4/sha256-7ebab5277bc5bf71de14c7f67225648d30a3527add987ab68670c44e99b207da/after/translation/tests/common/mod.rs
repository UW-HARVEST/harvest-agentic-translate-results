//! Shared harness: loads both the C reference `.so` and the Rust `.so` and
//! exposes the `rgb_to_hsv` export from each so they can be compared through
//! an identical FFI boundary.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

/// `void rgb_to_hsv(float *dest, const float *src)`
pub type RgbToHsvFn = unsafe extern "C" fn(*mut f32, *const f32);

pub struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: RgbToHsvFn,
    pub rust: RgbToHsvFn,
}

fn workspace_root() -> PathBuf {
    // .../<root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation dir has a parent")
        .to_path_buf()
}

/// Directory holding the freshly built Rust artifacts (`target/debug` or
/// `target/release`), derived from the running test binary's location
/// (`target/<profile>/deps/<test>`).
fn rust_artifact_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(Path::parent)
        .expect("test binary lives in target/<profile>/deps")
        .to_path_buf()
}

fn try_find_so(dir: &Path, must_contain: Option<&str>) -> Option<PathBuf> {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("so"))
        .filter(|p| match must_contain {
            None => true,
            Some(frag) => p
                .file_name()
                .and_then(|s| s.to_str())
                .map(|n| n.contains(frag))
                .unwrap_or(false),
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

fn find_so(dir: &Path, must_contain: Option<&str>) -> PathBuf {
    try_find_so(dir, must_contain).unwrap_or_else(|| {
        panic!(
            "no matching .so in {} (filter {:?})",
            dir.display(),
            must_contain
        )
    })
}

/// The C library must already be built:
///
/// ```text
/// cd c_src && mkdir -p build && cd build && \
///   cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
/// ```
fn c_so_path() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    assert!(
        build.is_dir(),
        "c_src/build missing - build the C reference library first"
    );
    find_so(&build, None)
}

fn rust_so_path() -> PathBuf {
    // `[lib] name = "rgb_to_hsv_lib"` -> librgb_to_hsv_lib.so
    let dir = rust_artifact_dir();
    if let Some(p) = try_find_so(&dir, Some("rgb_to_hsv_lib")) {
        return p;
    }
    // `cargo test` only builds the test binaries, not the `cdylib` artifact, so
    // build it here (into the same profile directory) and look again. The
    // cargo build lock is already released by the time tests run.
    build_cdylib(&dir);
    find_so(&dir, Some("rgb_to_hsv_lib"))
}

fn build_cdylib(artifact_dir: &Path) {
    use std::sync::OnceLock;
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        let release = artifact_dir
            .file_name()
            .and_then(|s| s.to_str())
            .map(|n| n == "release")
            .unwrap_or(false);
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = std::process::Command::new(cargo);
        cmd.current_dir(env!("CARGO_MANIFEST_DIR")).arg("build");
        if release {
            cmd.arg("--release");
        }
        let status = cmd.status().expect("spawn cargo build for the cdylib");
        assert!(status.success(), "cargo build of the cdylib failed");
    });
}

impl Impls {
    pub fn load() -> Impls {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("load C .so");
            let rust_lib = Library::new(rust_so_path()).expect("load Rust .so");

            let c_sym: Symbol<RgbToHsvFn> =
                c_lib.get(b"rgb_to_hsv\0").expect("C exports rgb_to_hsv");
            let rust_sym: Symbol<RgbToHsvFn> = rust_lib
                .get(b"rgb_to_hsv\0")
                .expect("Rust .so exports rgb_to_hsv");

            let c = *c_sym;
            let rust = *rust_sym;

            Impls {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }

    /// Runs both implementations on `src`, using a sentinel-filled 6-float
    /// destination buffer so out-of-range writes are also caught.
    pub fn call_both(&self, src: [f32; 3]) -> ([u32; 6], [u32; 6]) {
        const SENTINEL: f32 = -12345.678;
        let mut c_dest = [SENTINEL; 6];
        let mut rust_dest = [SENTINEL; 6];
        unsafe {
            (self.c)(c_dest.as_mut_ptr(), src.as_ptr());
            (self.rust)(rust_dest.as_mut_ptr(), src.as_ptr());
        }
        (bits(&c_dest), bits(&rust_dest))
    }

    /// Byte-for-byte comparison of both outputs for one input triple.
    pub fn assert_match(&self, src: [f32; 3]) {
        let (c, rust) = self.call_both(src);
        assert_eq!(
            c,
            rust,
            "mismatch for src = [{:e}, {:e}, {:e}] (bits {:#010x} {:#010x} {:#010x})\n  C    = {:#010x?}\n  Rust = {:#010x?}",
            src[0],
            src[1],
            src[2],
            src[0].to_bits(),
            src[1].to_bits(),
            src[2].to_bits(),
            c,
            rust
        );
    }
}

fn bits(v: &[f32; 6]) -> [u32; 6] {
    let mut out = [0u32; 6];
    for (o, f) in out.iter_mut().zip(v.iter()) {
        *o = f.to_bits();
    }
    out
}

/// Small deterministic xorshift PRNG so runs are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }

    /// Uniform in [0, 1].
    pub fn unit(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    /// Arbitrary bit pattern reinterpreted as `f32` (may be NaN/inf/subnormal).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}
