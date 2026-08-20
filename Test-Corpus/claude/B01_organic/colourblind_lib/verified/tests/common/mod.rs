//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as *shared objects* through `libloading` and
//! called only through their exported `colourblind` symbol. The Rust functions
//! are never called directly, so the `#[no_mangle] extern "C"` wrapper is
//! exercised exactly as an external C consumer would exercise it.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// The exported C ABI: `void colourblind(cb_impairment, float*, float*, float*)`.
/// `cb_impairment` is passed as a 32-bit integer.
pub type ColourblindFn = unsafe extern "C" fn(i32, *mut f32, *mut f32, *mut f32);

pub const CB_PROTANOPIA: i32 = 0;
pub const CB_DEUTERANOPIA: i32 = 1;
pub const CB_TRITANOPIA: i32 = 2;

/// All three valid `cb_impairment` enumerators.
pub const VALID_IMPAIRMENTS: [i32; 3] = [CB_PROTANOPIA, CB_DEUTERANOPIA, CB_TRITANOPIA];

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `target/<profile>/` — derived from the running test binary
/// (`target/<profile>/deps/<test>-<hash>`), so it is correct for both the
/// `dev` and `release` profiles.
fn target_profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test binary>
    exe.parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf()
}

/// Path to the Rust `cdylib` (`[lib] name = "colourblind_lib"`).
///
/// `cargo test` alone does **not** build a `cdylib`-only lib target (integration
/// tests do not link it), so if the artifact is missing we build it on demand.
pub fn rust_so_path() -> PathBuf {
    let dir = target_profile_dir();
    let p = dir.join("libcolourblind_lib.so");
    if !p.exists() {
        build_rust_cdylib(&dir);
    }
    assert!(
        p.exists(),
        "Rust cdylib not found at {}. Run `cargo build` first \
         (a `cdylib`-only lib target is not built by `cargo test`).",
        p.display()
    );
    p
}

/// Build the Rust `cdylib` for the profile the tests are running under.
/// Safe to call while `cargo test` is executing: the build lock is released
/// before test binaries run.
fn build_rust_cdylib(profile_dir: &Path) {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let profile = profile_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("debug");

    let mut cmd = Command::new(cargo);
    cmd.arg("build").arg("--lib").current_dir(manifest_dir());
    if profile == "release" {
        cmd.arg("--release");
    } else if profile != "debug" {
        cmd.args(["--profile", profile]);
    }
    // Forward the feature selection the tests were compiled with so the
    // cdylib under test matches the harness's expectations.
    if let Ok(features) = std::env::var("CB_TEST_FEATURES") {
        cmd.arg("--no-default-features");
        if !features.is_empty() {
            cmd.args(["--features", &features]);
        }
    }
    let _ = cmd.status();
}

/// Path to the C shared object produced by `c_src/CMakeLists.txt`.
///
/// The library name comes from the *parent directory* of `c_src` via
/// `cmake_path(GET parent FILENAME project_name)`, hence `libtranslated_rust.so`
/// for this checkout. Any `lib*.so` in the build dir is accepted so the harness
/// keeps working if the checkout is renamed.
pub fn c_so_path() -> PathBuf {
    let build_dir = manifest_dir().join("c_src/build");

    if !build_dir.exists() {
        build_c_library(&build_dir);
    }

    if let Some(p) = find_so(&build_dir) {
        return p;
    }

    // Present but empty / stale: try building once more.
    build_c_library(&build_dir);
    find_so(&build_dir).unwrap_or_else(|| {
        panic!(
            "no C shared object found in {}. Build it with:\n  cd c_src && mkdir -p build \
             && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

fn find_so(build_dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(build_dir).ok()?;
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|e| e == "so")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("lib"))
        })
        .collect();
    found.sort();
    found.into_iter().next()
}

/// Best-effort `cmake` build of the C library. Never modifies `c_src` sources.
fn build_c_library(build_dir: &Path) {
    let c_src = manifest_dir().join("c_src");
    let _ = std::fs::create_dir_all(build_dir);
    let _ = Command::new("cmake")
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(build_dir)
        .status();
    let _ = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(build_dir)
        .status();
    let _ = c_src;
}

/// A loaded library plus its resolved `colourblind` entry point.
pub struct Lib {
    /// Kept alive so the resolved symbol stays valid.
    _lib: Library,
    func: ColourblindFn,
    pub name: &'static str,
}

impl Lib {
    fn open(path: &Path, name: &'static str) -> Lib {
        // SAFETY: loading a plain C shared object with no initialisers of note.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        let func = {
            // SAFETY: the symbol's type matches the declared C prototype.
            let sym: Symbol<ColourblindFn> = unsafe { lib.get(b"colourblind\0") }
                .unwrap_or_else(|e| panic!("`colourblind` missing from {}: {e}", path.display()));
            *sym
        };
        Lib {
            _lib: lib,
            func,
            name,
        }
    }

    /// Call `colourblind` with three distinct, properly aligned `float`s.
    pub fn call(&self, impairment: i32, rgb: [f32; 3]) -> [f32; 3] {
        let mut v = rgb;
        // SAFETY: three distinct, aligned, initialised f32s.
        unsafe {
            (self.func)(
                impairment,
                &mut v[0] as *mut f32,
                &mut v[1] as *mut f32,
                &mut v[2] as *mut f32,
            );
        }
        v
    }

    /// Raw call, for aliasing / null / wild-pointer configurations.
    ///
    /// # Safety
    /// Caller guarantees the pointers are appropriate for the given impairment.
    pub unsafe fn call_raw(&self, impairment: i32, r: *mut f32, g: *mut f32, b: *mut f32) {
        unsafe { (self.func)(impairment, r, g, b) }
    }

    pub fn raw_fn(&self) -> ColourblindFn {
        self.func
    }
}

static C_LIB: OnceLock<Lib> = OnceLock::new();
static RUST_LIB: OnceLock<Lib> = OnceLock::new();

pub fn c_lib() -> &'static Lib {
    C_LIB.get_or_init(|| Lib::open(&c_so_path(), "C"))
}

pub fn rust_lib() -> &'static Lib {
    RUST_LIB.get_or_init(|| Lib::open(&rust_so_path(), "Rust"))
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
// ---------------------------------------------------------------------------

/// Bit-for-bit equality: distinguishes `+0.0` from `-0.0` and compares NaN
/// sign and payload bits exactly.
pub fn bits_eq(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|i| a[i].to_bits() == b[i].to_bits())
}

pub fn fmt3(v: [f32; 3]) -> String {
    format!(
        "[{:?} (0x{:08x}), {:?} (0x{:08x}), {:?} (0x{:08x})]",
        v[0],
        v[0].to_bits(),
        v[1],
        v[1].to_bits(),
        v[2],
        v[2].to_bits()
    )
}

/// Run one differential case with three distinct pointers and assert bit
/// equality. Returns the shared output on success.
pub fn assert_same(row: &str, impairment: i32, input: [f32; 3]) -> [f32; 3] {
    let c = c_lib().call(impairment, input);
    let r = rust_lib().call(impairment, input);
    assert!(
        bits_eq(c, r),
        "[{row}] divergence for Impairment={impairment} input={}\n  C   : {}\n  Rust: {}",
        fmt3(input),
        fmt3(c),
        fmt3(r)
    );
    c
}

/// Run a whole batch of inputs for one `CONFIGS.md` row.
pub fn run_row(row: &str, impairment: i32, inputs: &[[f32; 3]]) {
    assert!(!inputs.is_empty(), "[{row}] generated no inputs");
    for &input in inputs {
        assert_same(row, impairment, input);
    }
    eprintln!(
        "[{row}] OK  Impairment={impairment}  {} inputs",
        inputs.len()
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
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

    /// Uniform in `[-1, 1)`.
    pub fn signed_unit(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    pub fn bool(&mut self) -> bool {
        self.next_u32() & 1 == 1
    }

    /// A finite normal `f32` with a uniformly random exponent (full range) and
    /// random mantissa and sign.
    pub fn normal_full_range(&mut self) -> f32 {
        let sign = (self.next_u32() & 1) << 31;
        // biased exponent 1..=254 -> normal, not inf/nan
        let exp = 1 + self.below(254);
        let mantissa = self.next_u32() & 0x007F_FFFF;
        f32::from_bits(sign | (exp << 23) | mantissa)
    }

    /// A subnormal `f32` (biased exponent 0, non-zero mantissa) with random sign.
    pub fn subnormal(&mut self) -> f32 {
        let sign = (self.next_u32() & 1) << 31;
        let mantissa = 1 + (self.next_u32() & 0x007F_FFFE);
        f32::from_bits(sign | mantissa)
    }

    /// Any `f32` bit pattern at all: normals, subnormals, zeros, infinities,
    /// quiet and signalling NaNs.
    pub fn any_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}

// ---------------------------------------------------------------------------
// f32 helpers
// ---------------------------------------------------------------------------

/// Next representable `f32` towards `+inf` (bit-increment; NaN/inf passthrough).
pub fn next_up(v: f32) -> f32 {
    if v.is_nan() || v == f32::INFINITY {
        return v;
    }
    if v == 0.0 {
        return f32::from_bits(1); // smallest positive subnormal
    }
    let b = v.to_bits();
    if v > 0.0 {
        f32::from_bits(b + 1)
    } else {
        f32::from_bits(b - 1)
    }
}

/// Next representable `f32` towards `-inf`.
pub fn next_down(v: f32) -> f32 {
    if v.is_nan() || v == f32::NEG_INFINITY {
        return v;
    }
    if v == 0.0 {
        return f32::from_bits(0x8000_0001); // smallest negative subnormal
    }
    let b = v.to_bits();
    if v > 0.0 {
        f32::from_bits(b - 1)
    } else {
        f32::from_bits(b + 1)
    }
}

/// `2^k` as an exact `f32` for `k` in `-149..=127` (subnormal for `k < -126`).
pub fn pow2(k: i32) -> f32 {
    if k >= -126 {
        f32::from_bits((((k + 127) as u32) << 23) & 0x7F80_0000)
    } else {
        // subnormal: 2^-149 is bit pattern 1
        f32::from_bits(1u32 << (k + 149) as u32)
    }
}

/// The four NaN variants used by the payload-propagation rows:
/// positive quiet, negative quiet, quiet with a distinctive payload, and a
/// signalling NaN.
pub const NAN_VARIANTS: [u32; 4] = [
    0x7FC0_0000, // +qNaN (default)
    0xFFC0_0000, // -qNaN
    0x7FD5_5555, // +qNaN, distinctive payload
    0x7F80_0001, // +sNaN
];

/// Every combination of three values drawn from `pool` (`pool.len()^3` triples).
pub fn cube(pool: &[f32]) -> Vec<[f32; 3]> {
    let mut out = Vec::with_capacity(pool.len().pow(3));
    for &r in pool {
        for &g in pool {
            for &b in pool {
                out.push([r, g, b]);
            }
        }
    }
    out
}
