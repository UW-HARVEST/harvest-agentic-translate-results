//! Shared harness for the differential tests.
//!
//! Both implementations are loaded as *shared objects* through `libloading` and
//! called only through their exported `hsl_to_rgb` symbol. The Rust function is
//! never called directly, so the `#[no_mangle] extern "C"` wrapper is part of
//! what is under test.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

/// `void hsl_to_rgb(float *dest, const float *src)`
pub type HslToRgb = unsafe extern "C" fn(*mut f32, *const f32);

/// Workspace root (the directory holding `c_src/` and `translation/`).
fn root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// The C shared object. Its file name is derived from the *parent directory*
/// name by `c_src/CMakeLists.txt`, so it is discovered rather than hard-coded.
fn c_so_path() -> PathBuf {
    let build = root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}. Build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "so"))
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one .so in {}, found {found:?}",
        build.display()
    );
    found.pop().unwrap()
}

/// The Rust shared object. `RUST_SO` overrides; otherwise prefer `release`,
/// fall back to `debug`.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "RUST_SO={} is not a file", p.display());
        return p;
    }
    let base = root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libhsl_to_rgb_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "no libhsl_to_rgb_lib.so under {}; run `cargo build --release`",
        base.display()
    );
}

/// Both libraries plus the resolved entry points.
pub struct Pair {
    // Kept alive so the resolved function pointers stay valid.
    _c_lib: Library,
    _rust_lib: Library,
    pub c: HslToRgb,
    pub rust: HslToRgb,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

impl Pair {
    pub fn load() -> Pair {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        unsafe {
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display()));
            let c_sym: Symbol<HslToRgb> = c_lib
                .get(b"hsl_to_rgb\0")
                .expect("C .so does not export hsl_to_rgb");
            let rust_sym: Symbol<HslToRgb> = rust_lib
                .get(b"hsl_to_rgb\0")
                .expect("Rust .so does not export hsl_to_rgb");
            let c = *c_sym;
            let rust = *rust_sym;
            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
                c_path,
                rust_path,
            }
        }
    }

    /// Call C with `src` on a fresh, poisoned `dest`; return the raw bits.
    pub fn call_c(&self, src: [f32; 3]) -> [u32; 3] {
        call_one(self.c, src)
    }

    /// Call Rust with `src` on a fresh, poisoned `dest`; return the raw bits.
    pub fn call_rust(&self, src: [f32; 3]) -> [u32; 3] {
        call_one(self.rust, src)
    }
}

/// Poison pattern written into `dest` before every call, so an implementation
/// that fails to write a lane is caught instead of silently matching.
const POISON: u32 = 0xDEAD_BEEF;

fn call_one(f: HslToRgb, src: [f32; 3]) -> [u32; 3] {
    let mut dest = [f32::from_bits(POISON); 3];
    unsafe { f(dest.as_mut_ptr(), src.as_ptr()) };
    [
        dest[0].to_bits(),
        dest[1].to_bits(),
        dest[2].to_bits(),
    ]
}

/// Human-readable rendering of an `f32` that is unambiguous for `NaN`s and
/// signed zeros (which is the whole point of this test suite).
pub fn show(bits: u32) -> String {
    let v = f32::from_bits(bits);
    if v.is_nan() {
        let sign = if bits >> 31 == 1 { '-' } else { '+' };
        format!("{sign}NaN(0x{:06x}) [0x{bits:08x}]", bits & 0x007f_ffff)
    } else {
        format!("{v:e} [0x{bits:08x}]")
    }
}

pub fn show3(b: [u32; 3]) -> String {
    format!("[{}, {}, {}]", show(b[0]), show(b[1]), show(b[2]))
}

pub fn show_in(src: [f32; 3]) -> String {
    format!(
        "h={} s={} l={}",
        show(src[0].to_bits()),
        show(src[1].to_bits()),
        show(src[2].to_bits())
    )
}

/// Differential assertion: identical bit patterns out of both `.so`s.
///
/// `label` names the `CONFIGS.md` / `ERRORS.md` row being exercised.
#[track_caller]
pub fn assert_same(p: &Pair, label: &str, src: [f32; 3]) {
    let c = p.call_c(src);
    let r = p.call_rust(src);
    assert_eq!(
        c,
        r,
        "\n[{label}] divergence\n  input : {}\n  C     : {}\n  Rust  : {}\n",
        show_in(src),
        show3(c),
        show3(r)
    );
    for (i, bits) in c.iter().enumerate() {
        assert_ne!(
            *bits, POISON,
            "[{label}] C left dest[{i}] unwritten for {}",
            show_in(src)
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed so failures are reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn seeded() -> Rng {
        Rng(SEED)
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

    /// Any `f32` bit pattern at all: normals, subnormals, zeros, infinities
    /// and `NaN`s of both signs and arbitrary payload.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// A `NaN` with a random payload and a chosen sign. Payload is forced
    /// non-zero so the value really is a `NaN` and not an infinity.
    pub fn nan(&mut self, negative: bool) -> f32 {
        let payload = (self.next_u32() & 0x007f_ffff) | 1;
        let sign = if negative { 0x8000_0000 } else { 0 };
        f32::from_bits(sign | 0x7f80_0000 | payload)
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// Pick one element of a slice.
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u32() as usize) % xs.len()]
    }
}

// ---------------------------------------------------------------------------
// Shared value catalogues (axis definitions from CONFIGS.md)
// ---------------------------------------------------------------------------

/// One representative `h` from each of the nine reachable hue regions
/// (axis H in `CONFIGS.md`), used whenever a row says "all 9 regions".
pub const HUE_REGIONS: [(&str, f32, f32); 9] = [
    ("h<0", -1.0e9, -1.0e-3),
    ("[0,60)", 0.0, 60.0),
    ("[60,120)", 60.0, 120.0),
    ("[120,180) dead", 120.0, 180.0),
    ("[180,240)", 180.0, 240.0),
    ("[240,300)", 240.0, 300.0),
    ("[300,360)", 300.0, 360.0),
    (">=360", 360.0, 1.0e9),
    ("subnormal-ish", 0.0, 1.0e-30),
];

/// Sample an `h` inside region `i` of [`HUE_REGIONS`].
pub fn hue_in_region(rng: &mut Rng, i: usize) -> f32 {
    let (_, lo, hi) = HUE_REGIONS[i];
    rng.range(lo, hi)
}

/// Exact `f32` sector boundaries the cascade compares against.
pub const SECTOR_EDGES: [f32; 7] = [0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0];

/// Interesting `f32` values that are not tied to the hue cascade.
pub const SPECIALS: [f32; 21] = [
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    // smallest positive subnormal and a mid subnormal
    1.0e-45,
    -1.0e-45,
    f32::MAX,
    f32::MIN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    f32::EPSILON,
    -f32::EPSILON,
    16_777_216.0, // 2^24, last exactly-representable integer step
    -16_777_216.0,
    1.0e30,
];
