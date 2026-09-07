//! Shared harness: loads BOTH the C `.so` and the Rust `.so` with `libloading`
//! and calls `hsl_to_rgb` through the FFI boundary on each of them.
//!
//! The Rust implementation is NEVER called directly — always through the
//! dynamically loaded cdylib, so the `#[no_mangle] extern "C"` wrapper is
//! itself under test.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

pub type HslToRgb = unsafe extern "C" fn(*mut f32, *const f32);

pub struct Libs {
    _c: Library,
    _rust: Library,
    pub c: HslToRgb,
    pub rust: HslToRgb,
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn find_one(dir: &Path, pred: impl Fn(&str) -> bool) -> PathBuf {
    let mut hits: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(&pred)
                .unwrap_or(false)
        })
        .collect();
    hits.sort();
    match hits.len() {
        0 => panic!("no matching .so in {}", dir.display()),
        _ => hits.remove(0),
    }
}

fn c_so() -> PathBuf {
    // Override lets run_all.sh re-run the whole suite against a C library built
    // with different optimisation settings (-O0 vs -O2), which is where
    // floating-point operand-order / NaN-propagation differences would show up.
    if let Ok(p) = std::env::var("HSL_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "HSL_C_SO points at {}, which does not exist", p.display());
        return p;
    }
    let build = repo_root().join("c_src/build");
    assert!(
        build.is_dir(),
        "c_src/build missing — build the C library first:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    find_one(&build, |n| n.starts_with("lib") && n.ends_with(".so"))
}

fn rust_so() -> PathBuf {
    // Explicit override wins (used by run_all.sh when cross-checking builds).
    if let Ok(p) = std::env::var("HSL_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "HSL_RUST_SO points at {}, which does not exist", p.display());
        return p;
    }
    // The integration test binary lives in target/<profile>/deps/, so the
    // cdylib we want is two levels up — i.e. the SAME profile as this test.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();
    let direct = profile_dir.join("libhsl_to_rgb_lib.so");
    assert!(
        direct.is_file(),
        "the Rust cdylib {} does not exist.\n`cargo test` does not build \
         crate-type=cdylib artifacts, so build it first with the SAME profile \
         and features, e.g.:\n  cargo build --offline{}\nor point HSL_RUST_SO \
         at the .so explicitly.",
        direct.display(),
        if profile_dir.ends_with("release") { " --release" } else { "" }
    );
    direct
}

impl Libs {
    pub fn load() -> Libs {
        unsafe {
            let cpath = c_so();
            let rpath = rust_so();
            let clib = Library::new(&cpath)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", cpath.display()));
            let rlib = Library::new(&rpath)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rpath.display()));
            let c = {
                let s: Symbol<HslToRgb> = clib
                    .get(b"hsl_to_rgb\0")
                    .expect("hsl_to_rgb missing from C .so");
                *s
            };
            let rust = {
                let s: Symbol<HslToRgb> = rlib
                    .get(b"hsl_to_rgb\0")
                    .expect("hsl_to_rgb missing from Rust .so");
                *s
            };
            Libs {
                _c: clib,
                _rust: rlib,
                c,
                rust,
            }
        }
    }

    /// Call both libraries on `src` (disjoint output buffers) and return
    /// `(c_out, rust_out)`.
    pub fn call_both(&self, src: [f32; 3]) -> ([f32; 3], [f32; 3]) {
        // Poison the destinations differently so a missing store is detected.
        let mut co = [f32::from_bits(0xDEAD_BEEF); 3];
        let mut ro = [f32::from_bits(0xDEAD_BEEF); 3];
        unsafe {
            (self.c)(co.as_mut_ptr(), src.as_ptr());
            (self.rust)(ro.as_mut_ptr(), src.as_ptr());
        }
        (co, ro)
    }

    /// Assert bit-for-bit equality of the three outputs for `src`.
    pub fn assert_same(&self, src: [f32; 3], ctx: &str) {
        let (co, ro) = self.call_both(src);
        if bits(&co) != bits(&ro) {
            panic!(
                "DIVERGENCE [{ctx}]\n  src  = {}\n  C    = {}\n  Rust = {}",
                show(&src),
                show(&co),
                show(&ro)
            );
        }
    }
}

pub fn bits(v: &[f32; 3]) -> [u32; 3] {
    [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()]
}

pub fn show(v: &[f32; 3]) -> String {
    format!(
        "[{:?}(0x{:08x}), {:?}(0x{:08x}), {:?}(0x{:08x})]",
        v[0],
        v[0].to_bits(),
        v[1],
        v[1].to_bits(),
        v[2],
        v[2].to_bits()
    )
}

/// Deterministic xorshift64* PRNG — fixed seed, reproducible everywhere.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_DEAD_BEEF;

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 1 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
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
    /// A completely arbitrary bit pattern reinterpreted as `f32`
    /// (may be NaN, Inf, subnormal, huge, negative zero...).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// A NaN with a random sign and a random non-zero payload.
    pub fn any_nan(&mut self) -> f32 {
        let r = self.next_u32();
        let sign = r & 0x8000_0000;
        let payload = (r & 0x007F_FFFF).max(1);
        f32::from_bits(sign | 0x7F80_0000 | payload)
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u32() as usize) % xs.len()]
    }
}

/// The seven dispatch regions of the hue argument, as branched on by the C.
/// `H4` (`[120,180)`) is included because the C's doubled `h < 120.0f` test
/// makes that arm unreachable — it must fall through to the final `else`.
pub const HUE_REGIONS: &[(&str, f32, f32)] = &[
    ("H1 [0,60)", 0.0, 60.0),
    ("H2 [60,120)", 60.0, 120.0),
    ("H4 [120,180) unreachable-arm", 120.0, 180.0),
    ("H5 [180,240)", 180.0, 240.0),
    ("H6 [240,300)", 240.0, 300.0),
    ("H7 [300,360)", 300.0, 360.0),
    ("H8 [360,1e4)", 360.0, 10_000.0),
    ("H3 negative", -1_000.0, -0.0001),
];

/// Interesting lightness shapes (axis L).
pub const L_SHAPES: &[(&str, f32, f32)] = &[
    ("L1 l<0.5", 0.0, 0.5),
    ("L3 l>0.5", 0.5, 1.0),
    ("L5 l<0", -5.0, -0.001),
    ("L5 l>1", 1.001, 5.0),
];

