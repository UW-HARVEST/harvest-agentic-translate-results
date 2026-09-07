//! Shared differential-test harness.
//!
//! Both libraries are loaded as shared objects via `libloading` and called
//! through their exported `tritanopia` symbol. The Rust implementation is
//! NEVER called directly as a Rust function - always through the `.so`, so the
//! `#[no_mangle] extern "C"` wrapper and the struct ABI are under test too.

#![allow(dead_code)] // each test file uses a different subset of the harness

use libloading::{Library, Symbol};
use std::path::PathBuf;

/// Mirrors `typedef struct cb_rgb_255 { unsigned char R, G, B; }`.
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct CbRgb255 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl CbRgb255 {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

pub type TritanopiaFn = unsafe extern "C" fn(CbRgb255) -> CbRgb255;
/// Same function seen through the raw register ABI, for the padding-bits test.
pub type TritanopiaRawFn = unsafe extern "C" fn(u64) -> u64;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}). Build the C first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one .so in {}, found {:?}",
        build.display(),
        candidates
    );
    candidates.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    // IMPORTANT: `cargo test` does NOT rebuild a `crate-type = ["cdylib"]`
    // artifact - it only builds the test harness. So the `.so` on disk can be
    // STALE relative to src/lib.rs, and the whole differential suite would then
    // be silently validating an old binary (verified: an injected mutation
    // passed every test until this guard was added). Refuse to run in that
    // case rather than reporting a meaningless pass.
    let root = workspace_root().join("translation/target");
    let mut newest: Option<PathBuf> = None;
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("libtritanopia_lib.so");
        if p.exists() {
            newest = Some(p);
            break;
        }
    }
    let so = newest.unwrap_or_else(|| {
        panic!(
            "libtritanopia_lib.so not found under {}. Run `cargo build --release` first.",
            root.display()
        )
    });

    let src = workspace_root().join("translation/src/lib.rs");
    let so_t = std::fs::metadata(&so).and_then(|m| m.modified()).ok();
    let src_t = std::fs::metadata(&src).and_then(|m| m.modified()).ok();
    if let (Some(so_t), Some(src_t)) = (so_t, src_t) {
        assert!(
            so_t >= src_t,
            "STALE ARTIFACT: {} is older than {}.\n\
             `cargo test` does not rebuild a cdylib, so this run would have tested \
             an out-of-date library. Run `cargo build --release --offline` (or \
             ./run_all.sh) before `cargo test`.",
            so.display(),
            src.display()
        );
    }
    so
}

/// Both libraries, kept alive for the lifetime of the test.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c: TritanopiaFn,
    rust: TritanopiaFn,
    c_raw: TritanopiaRawFn,
    rust_raw: TritanopiaRawFn,
}

impl Pair {
    pub fn load() -> Self {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));

            let c: Symbol<TritanopiaFn> = c_lib
                .get(b"tritanopia\0")
                .expect("C .so does not export `tritanopia`");
            let rust: Symbol<TritanopiaFn> = rust_lib
                .get(b"tritanopia\0")
                .expect("Rust .so does not export `tritanopia`");
            let c_raw: Symbol<TritanopiaRawFn> = c_lib.get(b"tritanopia\0").unwrap();
            let rust_raw: Symbol<TritanopiaRawFn> = rust_lib.get(b"tritanopia\0").unwrap();

            Self {
                c: *c,
                rust: *rust,
                c_raw: *c_raw,
                rust_raw: *rust_raw,
                _c_lib: c_lib,
                _rust_lib: rust_lib,
            }
        }
    }

    pub fn c(&self, v: CbRgb255) -> CbRgb255 {
        unsafe { (self.c)(v) }
    }

    pub fn rust(&self, v: CbRgb255) -> CbRgb255 {
        unsafe { (self.rust)(v) }
    }

    pub fn c_raw(&self, v: u64) -> u64 {
        unsafe { (self.c_raw)(v) }
    }

    pub fn rust_raw(&self, v: u64) -> u64 {
        unsafe { (self.rust_raw)(v) }
    }

    /// Compare one input; returns the shared output on success.
    #[track_caller]
    pub fn check(&self, v: CbRgb255) -> CbRgb255 {
        let c = self.c(v);
        let r = self.rust(v);
        assert_eq!(
            c, r,
            "DIVERGENCE for input (R={}, G={}, B={}): C={:?} Rust={:?}",
            v.r, v.g, v.b, c, r
        );
        c
    }

    /// Compare every input produced by `inputs`, reporting the row name.
    #[track_caller]
    pub fn check_all<I: IntoIterator<Item = CbRgb255>>(&self, row: &str, inputs: I) -> usize {
        let mut n = 0usize;
        let mut failures = Vec::new();
        for v in inputs {
            let c = self.c(v);
            let r = self.rust(v);
            if c != r && failures.len() < 10 {
                failures.push((v, c, r));
            }
            n += 1;
        }
        assert!(
            failures.is_empty(),
            "[{row}] {} of {n} inputs diverged; first {}: {:?}",
            failures.len(),
            failures.len(),
            failures
        );
        assert!(n > 0, "[{row}] generated no inputs");
        n
    }
}

/// SplitMix64 - deterministic, seeded, no external dependency.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

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

    /// Uniform in `lo..=hi`.
    pub fn range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u8
    }
}

/// The remove-gamma branch boundary: `byte/255 > 0.04045` <=> `byte >= 11`.
pub const GAMMA_LINEAR_MAX: u8 = 10;
pub const GAMMA_POW_MIN: u8 = 11;

/// Recompute the C's pre-`cbDenorm` float for a channel, for classifying which
/// `cbDenorm` outcome a given input exercises (used to prove coverage of the
/// negative-wrap / over-255-wrap rows). Mirrors `lib.c` exactly.
pub fn pre_denorm(v: CbRgb255) -> [f32; 3] {
    fn remove_gamma(c: f32) -> f32 {
        let c = c as f64;
        let v = if c > 0.04045 {
            ((c + 0.055) / 1.055).powf(2.4)
        } else {
            c / 12.92
        };
        v as f32
    }
    fn apply_gamma(c: f32) -> f32 {
        let c = c as f64;
        let v = if c > 0.00313080495356037151702786377709 {
            1.055 * c.powf(0.4166666666) - 0.055
        } else {
            c * 12.92
        };
        v as f32
    }
    let r = remove_gamma(v.r as f32 / 255.0);
    let g = remove_gamma(v.g as f32 / 255.0);
    let b = remove_gamma(v.b as f32 / 255.0);
    let nr = r + 0.12739886310880f32 * g - 0.12739886341072f32 * b;
    let ng = -4.486E-11f32 * r + 0.87390929928361f32 * g + 0.12609070101523f32 * b;
    let nb = 3.1113E-10f32 * r + 0.87390929725848f32 * g + 0.12609070067115f32 * b;
    [
        apply_gamma(nr) * 255.0 + 0.5,
        apply_gamma(ng) * 255.0 + 0.5,
        apply_gamma(nb) * 255.0 + 0.5,
    ]
}
