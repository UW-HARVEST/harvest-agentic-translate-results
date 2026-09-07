#![allow(dead_code)] // each test binary uses a different subset of this module

//! Shared harness: loads BOTH shared objects via `libloading` and calls
//! `ldexp_q2` through the FFI boundary in both.
//!
//! Nothing in here calls the Rust implementation directly — the Rust side is
//! always reached through `dlopen`/`dlsym` on `libldexp_q2_lib.so`, exactly as
//! an external C consumer would, so the `#[no_mangle] extern "C"` export
//! wrapper is under test too.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub type LdexpQ2 = unsafe extern "C" fn(f32, i32) -> f32;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn build_c_so() -> PathBuf {
    let root = workspace_root();
    let c_src = root.join("c_src");
    let build = c_src.join("build");

    if let Some(existing) = find_so(&build) {
        return existing;
    }

    std::fs::create_dir_all(&build).expect("create c_src/build");
    let ok = Command::new("cmake")
        .arg("..")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .current_dir(&build)
        .status()
        .expect("run cmake configure")
        .success();
    assert!(ok, "cmake configure failed");
    let ok = Command::new("cmake")
        .args(["--build", "."])
        .current_dir(&build)
        .status()
        .expect("run cmake build")
        .success();
    assert!(ok, "cmake build failed");

    find_so(&build).expect("C .so not produced by cmake build")
}

fn find_so(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut hits: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|s| s.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with("lib"))
        })
        .collect();
    hits.sort();
    hits.into_iter().next()
}

/// Locate the Rust `cdylib`. An explicit `RUST_SO` env var wins (used by
/// `scripts/verify_all.sh` to verify the debug **and** the optimized
/// `panic = "abort"` artifact); otherwise prefer the artifact next to the test
/// binary, otherwise build it.
fn rust_so() -> PathBuf {
    if let Some(p) = std::env::var_os("RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "RUST_SO={} is not a file", p.display());
        return p;
    }

    let root = workspace_root();
    let target = root.join("translation").join("target");
    let name = "libldexp_q2_lib.so";

    for profile in ["debug", "release"] {
        let p = target.join(profile).join(name);
        if p.is_file() {
            return p;
        }
    }

    let ok = Command::new(env!("CARGO"))
        .args(["build"])
        .current_dir(root.join("translation"))
        .status()
        .expect("cargo build for cdylib")
        .success();
    assert!(ok, "cargo build of the cdylib failed");

    let p = target.join("debug").join(name);
    assert!(p.is_file(), "Rust cdylib missing at {}", p.display());
    p
}

pub struct Pair {
    _c_lib: Library,
    _rs_lib: Library,
    pub c: LdexpQ2,
    pub rs: LdexpQ2,
    pub c_path: PathBuf,
    pub rs_path: PathBuf,
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        let c_path = build_c_so();
        let rs_path = rust_so();

        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rs_lib = Library::new(&rs_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rs_path.display()));

            let c_sym: Symbol<LdexpQ2> = c_lib
                .get(b"ldexp_q2\0")
                .expect("C .so must export ldexp_q2");
            let rs_sym: Symbol<LdexpQ2> = rs_lib
                .get(b"ldexp_q2\0")
                .expect("Rust .so must export ldexp_q2");

            let c = *c_sym;
            let rs = *rs_sym;

            Pair {
                _c_lib: c_lib,
                _rs_lib: rs_lib,
                c,
                rs,
                c_path,
                rs_path,
            }
        }
    })
}

/// Bit-exact differential check of one `(y, exp_q2)` pair.
///
/// Compares raw `f32` bit patterns, so `+0.0` vs `-0.0` and differing `NaN`
/// payloads are treated as divergences.
#[track_caller]
pub fn check(y: f32, exp_q2: i32) {
    let p = pair();
    let cv = unsafe { (p.c)(y, exp_q2) };
    let rv = unsafe { (p.rs)(y, exp_q2) };
    assert_eq!(
        cv.to_bits(),
        rv.to_bits(),
        "divergence: ldexp_q2(y=0x{:08x} ({:e}), exp_q2={}) -> C 0x{:08x} ({:e}) vs RS 0x{:08x} ({:e})",
        y.to_bits(),
        y,
        exp_q2,
        cv.to_bits(),
        cv,
        rv.to_bits(),
        rv,
    );
}

/// Deterministic, fixed-seed SplitMix64 so every "randomized" row is
/// reproducible.
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
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Random `f32` from raw bits: hits normals, subnormals, zeros, infinities
    /// and NaNs in their natural proportions.
    pub fn next_f32_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// Random *finite normal-ish* `f32` spanning the whole exponent range.
    pub fn next_f32_normal(&mut self) -> f32 {
        loop {
            let v = f32::from_bits(self.next_u32());
            if v.is_finite() && v != 0.0 {
                return v;
            }
        }
    }
    pub fn in_range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

/// Every interesting `f32` shape from axis 4 of `CONFIGS.md`.
pub const Y_CLASSES: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    2.0,
    -2.0,
    0.5,
    3.141_592_7,
    -3.141_592_7,
    f32::MIN_POSITIVE,             // FLT_MIN
    -f32::MIN_POSITIVE,
    f32::MAX,                      // FLT_MAX
    f32::MIN,                      // -FLT_MAX
    f32::EPSILON,
    f32::INFINITY,
    f32::NEG_INFINITY,
];

/// `y` values that can only be spelled as bit patterns (subnormals, NaN
/// payloads, signalling NaN).
pub const Y_BIT_CLASSES: &[u32] = &[
    0x0000_0001, // smallest positive subnormal
    0x8000_0001, // smallest negative subnormal
    0x007f_ffff, // largest positive subnormal
    0x807f_ffff, // largest negative subnormal
    0x0040_0000, // mid subnormal
    0x7f80_0000, // +inf
    0xff80_0000, // -inf
    0x7fc0_0000, // default quiet NaN
    0xffc0_0000, // negative quiet NaN
    0x7fc0_dead, // quiet NaN, custom payload
    0x7f80_0001, // signalling NaN
    0xff80_0001, // negative signalling NaN
    0x7f7f_ffff, // FLT_MAX
    0x0080_0000, // FLT_MIN
];

pub fn y_all() -> Vec<f32> {
    let mut v: Vec<f32> = Y_CLASSES.to_vec();
    v.extend(Y_BIT_CLASSES.iter().copied().map(f32::from_bits));
    v
}

/// `exp_q2` values that hit at least one member of every class in
/// `CONFIGS.md` axes 1–3.
///
/// Huge *positive* exponents are deliberately excluded (they cost
/// `exp_q2 / 120` loop iterations each); they live in
/// [`exp_huge_positive`] and are paired with a small `y` set so the suite
/// stays well inside the time budget.
pub fn exp_classes() -> Vec<i32> {
    let mut v = vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 15, 16, 63, 64, 118, 119, 120, 121, 122, 123, 124, 239, 240,
        241, 480, 1200, 100_000, -1, -2, -3, -4, -5, -6, -7, -8, -9, -12, -16, -120, -121, -124,
        -127, -128, -129, -130, -131, -132, -256, -257, -384, -508, -509, -511, -512, -100_000,
        i32::MIN, i32::MIN + 1, i32::MIN + 2, i32::MIN + 3, i32::MIN + 4,
    ];
    for k in 0..=30u32 {
        let p = 1i32 << k;
        // Positive powers only up to 2^17 to bound the iteration count.
        if k <= 17 {
            v.push(p);
            v.push(p - 1);
            v.push(p + 1);
        }
        v.push(-p);
        v.push(-p - 1);
        v.push(-p + 1);
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// The expensive positive tail of axis 1 (multi-million-iteration loops).
pub fn exp_huge_positive() -> Vec<i32> {
    vec![
        1 << 20,
        1 << 24,
        i32::MAX - 120,
        i32::MAX - 119,
        i32::MAX - 1,
        i32::MAX,
    ]
}

