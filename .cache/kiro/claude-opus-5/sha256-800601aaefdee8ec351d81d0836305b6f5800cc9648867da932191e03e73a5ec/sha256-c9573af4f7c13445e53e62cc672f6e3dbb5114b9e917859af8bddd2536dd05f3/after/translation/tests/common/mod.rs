//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls the exported
//! `rgb_to_hsv` symbol on each. The Rust implementation is NEVER called
//! directly as a Rust function — it is always reached through
//! `librgb_to_hsv_lib.so`'s `#[no_mangle]` export, exactly as an external C
//! consumer would reach it. That way the export wrapper itself is under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type RgbToHsvFn = unsafe extern "C" fn(*mut f32, *const f32);

/// Repository root (the directory holding both `c_src/` and `translation/`).
fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = repo_root().join("c_src").join("build");
    let mut found: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                found.push(p);
            }
        }
    }
    found.sort();
    found.pop().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // Explicit override, used by run_all.sh to test each build profile.
    if let Ok(p) = std::env::var("HARVEST_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "HARVEST_RUST_SO={} does not exist", p.display());
        return p;
    }
    // Prefer the release cdylib; fall back to debug.
    let target = repo_root().join("translation").join("target");
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("librgb_to_hsv_lib.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "librgb_to_hsv_lib.so not found under {}. Build it with: cd translation && cargo build --release",
        target.display()
    );
}

struct Libs {
    c: Library,
    rust: Library,
}

// Safety: both libraries are stateless (pure functions over caller buffers), so
// sharing the handles across test threads is sound.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        unsafe {
            let c = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", c_path.display()));
            let rust = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", rust_path.display()));
            Libs { c, rust }
        }
    })
}

/// `rgb_to_hsv` as exported by the C shared object.
pub fn c_fn() -> RgbToHsvFn {
    unsafe {
        let s: Symbol<RgbToHsvFn> = libs()
            .c
            .get(b"rgb_to_hsv\0")
            .expect("C .so does not export rgb_to_hsv");
        *s
    }
}

/// `rgb_to_hsv` as exported by the Rust shared object (via `#[no_mangle]`).
pub fn rust_fn() -> RgbToHsvFn {
    unsafe {
        let s: Symbol<RgbToHsvFn> = libs()
            .rust
            .get(b"rgb_to_hsv\0")
            .expect("Rust .so does not export rgb_to_hsv");
        *s
    }
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

/// Bitwise-exact representation of a triple, for comparison and reporting.
pub fn bits3(v: &[f32; 3]) -> [u32; 3] {
    [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()]
}

fn show(v: &[f32; 3]) -> String {
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

/// Call both libraries on `src` with disjoint destination buffers and assert
/// the three written `f32`s are bitwise identical.
///
/// Destination buffers are pre-filled with a poison pattern so that a failure
/// to write a slot is also caught.
#[track_caller]
pub fn assert_same(src: &[f32; 3], row: &str) {
    const POISON: f32 = -1.234_567_9e-17;
    let mut dc = [POISON; 3];
    let mut dr = [POISON; 3];

    let c = c_fn();
    let r = rust_fn();
    unsafe {
        c(dc.as_mut_ptr(), src.as_ptr());
        r(dr.as_mut_ptr(), src.as_ptr());
    }

    if bits3(&dc) != bits3(&dr) {
        panic!(
            "[{row}] divergence for src = [{:?} (0x{:08x}), {:?} (0x{:08x}), {:?} (0x{:08x})]\n  \
             C    = {}\n  Rust = {}",
            src[0],
            src[0].to_bits(),
            src[1],
            src[1].to_bits(),
            src[2],
            src[2].to_bits(),
            show(&dc),
            show(&dr)
        );
    }
}

/// Same as [`assert_same`] but also returns the (agreed) C output so a test can
/// additionally assert a specific expected value.
#[track_caller]
pub fn check(src: &[f32; 3], row: &str) -> [f32; 3] {
    assert_same(src, row);
    let mut dc = [0.0f32; 3];
    unsafe { c_fn()(dc.as_mut_ptr(), src.as_ptr()) };
    dc
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

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
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [lo, hi).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// A raw 32-bit pattern reinterpreted as f32 (may be NaN/inf/subnormal).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A random subnormal f32 with random sign.
    pub fn subnormal(&mut self) -> f32 {
        let mantissa = self.next_u32() & 0x007F_FFFF;
        let sign = (self.next_u32() & 1) << 31;
        f32::from_bits(sign | mantissa)
    }
}

/// The canonical seed used by every property-style test.
pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

/// How many randomized inputs each configuration row is driven with.
pub const ITERS: usize = 20_000;
