//! Shared harness for the differential tests.
//!
//! Both the C shared object and the Rust `cdylib` are loaded with `libloading`
//! and driven exclusively through their exported `hsv_to_rgb` symbol, so the
//! `#[no_mangle] extern "C"` wrapper is part of what is under test. No Rust
//! function is ever called directly.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// The one and only exported entry point.
pub type HsvToRgb = unsafe extern "C" fn(*mut f32, *const f32);

pub struct Libs {
    // Kept alive for the lifetime of the function pointers.
    _c_lib: Library,
    _rust_lib: Library,
    pub c: HsvToRgb,
    pub rust: HsvToRgb,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C `.so`. The CMake project name is derived from the parent
/// directory name, so the file name is not fixed — glob for it instead.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_C_SO") {
        return PathBuf::from(p);
    }
    let build_dir = manifest_dir().join("../c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", build_dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|e| e == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {} — build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

/// Locate the Rust `cdylib`. Prefers the release artifact (that is what ships),
/// falling back to whatever profile directory the test binary lives in.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        return PathBuf::from(p);
    }
    const NAME: &str = "libhsv_to_rgb_lib.so";
    let mut roots: Vec<PathBuf> = Vec::new();
    // target/<profile>/deps/<test-bin>  ->  target/<profile>
    if let Ok(exe) = std::env::current_exe() {
        if let Some(deps) = exe.parent() {
            if let Some(profile) = deps.parent() {
                roots.push(profile.to_path_buf());
                if let Some(target) = profile.parent() {
                    roots.push(target.join("release"));
                    roots.push(target.join("debug"));
                }
            }
        }
    }
    roots.push(manifest_dir().join("target/release"));
    roots.push(manifest_dir().join("target/debug"));

    // Release first so the optimised code path is the one being verified.
    roots.sort_by_key(|r| {
        let s = r.to_string_lossy().to_string();
        if s.ends_with("release") { 0 } else { 1 }
    });
    for r in &roots {
        let p = r.join(NAME);
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "{NAME} not found; run `cargo build --release` first (looked in {:?})",
        roots
    );
}

/// Fail loudly if `artifact` is older than `source`.
fn assert_fresh(artifact: &Path, source: &Path, rebuild_cmd: &str) {
    let m = |p: &Path| {
        std::fs::metadata(p)
            .and_then(|md| md.modified())
            .unwrap_or_else(|e| panic!("stat {}: {e}", p.display()))
    };
    if !source.is_file() {
        return; // nothing to compare against
    }
    let (a, s) = (m(artifact), m(source));
    assert!(
        a >= s,
        "STALE ARTIFACT: {} is older than {}.\n\
         `cargo test` does not rebuild a cdylib target, so the differential \
         tests would compare a stale library and pass vacuously.\n\
         Run `{rebuild_cmd}` first (or use scripts/run_all.sh).",
        artifact.display(),
        source.display()
    );
}

fn load(path: &Path) -> (Library, HsvToRgb) {    unsafe {
        let lib = Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()));
        let f: Symbol<HsvToRgb> = lib
            .get(b"hsv_to_rgb\0")
            .unwrap_or_else(|e| panic!("dlsym hsv_to_rgb in {}: {e}", path.display()));
        let raw = *f;
        (lib, raw)
    }
}

impl Libs {
    pub fn load() -> Self {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        // `cargo test` does NOT rebuild a `cdylib` target, so it is easy to
        // silently differential-test a stale `.so` and get a false pass.
        // Refuse to run unless both shared objects are newer than their source.
        assert_fresh(&rust_path, &manifest_dir().join("src/lib.rs"), "cargo build --release");
        assert_fresh(
            &c_path,
            &manifest_dir().join("../c_src/src/lib.c"),
            "cmake --build c_src/build",
        );
        let (c_lib, c) = load(&c_path);
        let (rust_lib, rust) = load(&rust_path);
        Libs {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
            c_path,
            rust_path,
        }
    }

    /// Call both libraries with `src` into freshly poisoned 3-float buffers and
    /// return the raw bit patterns.
    pub fn both(&self, src: [f32; 3]) -> ([u32; 3], [u32; 3]) {
        const POISON: f32 = f32::from_bits(0xDEAD_BEEF);
        let mut dc = [POISON; 3];
        let mut dr = [POISON; 3];
        let sc = src;
        let sr = src;
        unsafe {
            (self.c)(dc.as_mut_ptr(), sc.as_ptr());
            (self.rust)(dr.as_mut_ptr(), sr.as_ptr());
        }
        (bits3(&dc), bits3(&dr))
    }

    /// Differential assertion for one input triple.
    pub fn check(&self, row: &str, src: [f32; 3]) {
        let (c, r) = self.both(src);
        assert_bits_eq(row, src, c, r);
    }
}

pub fn bits3(v: &[f32; 3]) -> [u32; 3] {
    [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()]
}

pub fn assert_bits_eq(row: &str, src: [f32; 3], c: [u32; 3], r: [u32; 3]) {
    if c != r {
        panic!(
            "[{row}] DIVERGENCE\n  src  = [{:e} (0x{:08x}), {:e} (0x{:08x}), {:e} (0x{:08x})]\n  \
             C    = [0x{:08x} ({}), 0x{:08x} ({}), 0x{:08x} ({})]\n  \
             Rust = [0x{:08x} ({}), 0x{:08x} ({}), 0x{:08x} ({})]",
            src[0],
            src[0].to_bits(),
            src[1],
            src[1].to_bits(),
            src[2],
            src[2].to_bits(),
            c[0],
            f32::from_bits(c[0]),
            c[1],
            f32::from_bits(c[1]),
            c[2],
            f32::from_bits(c[2]),
            r[0],
            f32::from_bits(r[0]),
            r[1],
            f32::from_bits(r[1]),
            r[2],
            f32::from_bits(r[2]),
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

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
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
    /// An arbitrary 32-bit pattern reinterpreted as `f32` (any float class,
    /// including every NaN payload and both zeros).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}

/// The interesting float classes, used to build cross-products.
pub const SPECIAL_F32: &[f32] = &[
    0.0,
    -0.0,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    f32::from_bits(1),  // smallest positive subnormal
    f32::from_bits(0x8000_0001), // smallest negative subnormal
    1.0,
    -1.0,
    0.5,
    2.0,
    255.0,
    1e-30,
    1e30,
    -1e30,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    -f32::NAN,
    f32::from_bits(0x7FC0_1234), // quiet NaN with payload
    f32::from_bits(0x7F80_0001), // signalling NaN
];

/// Hue values that land in every sector, plus the pathological ones.
pub const SPECIAL_HUE: &[f32] = &[
    0.0,
    -0.0,
    30.0,
    59.999_996,
    60.0,
    90.0,
    120.0,
    150.0,
    180.0,
    210.0,
    240.0,
    270.0,
    300.0,
    330.0,
    359.999_97,
    360.0,
    420.0,
    -1.0,
    -60.0,
    -120.0,
    -1e6,
    1e6,
    1e30,
    -1e30,
    2147483648.0 * 60.0, // h/60 == 2^31 exactly
    -2147483648.0 * 60.0, // h/60 == -2^31 exactly
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::NAN,
    f32::from_bits(1),
];
