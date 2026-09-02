//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both shared objects are loaded with `libloading` and driven **only** through
//! their exported `colourblind` symbol, so the Rust `#[no_mangle] extern "C"`
//! wrapper is exercised exactly as an external C caller would exercise it.
//! Nothing in `translation/src/lib.rs` is called directly.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// `void colourblind(cb_impairment, float*, float*, float*)`
pub type CbFn = unsafe extern "C" fn(c_int, *mut f32, *mut f32, *mut f32);

pub const CB_PROTANOPIA: c_int = 0;
pub const CB_DEUTERANOPIA: c_int = 1;
pub const CB_TRITANOPIA: c_int = 2;
pub const VALID_IMPAIRMENTS: [c_int; 3] = [CB_PROTANOPIA, CB_DEUTERANOPIA, CB_TRITANOPIA];

pub fn impairment_name(i: c_int) -> &'static str {
    match i {
        CB_PROTANOPIA => "Protanopia",
        CB_DEUTERANOPIA => "Deuteranopia",
        CB_TRITANOPIA => "Tritanopia",
        _ => "<out of range>",
    }
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    // translation/ -> parent is the working directory holding c_src/ too.
    manifest_dir().parent().map(Path::to_path_buf).unwrap()
}

/// The C `.so`. Its file name is derived by CMake from the *parent directory
/// name* (`cmake_path(GET parent FILENAME project_name)`), so it is discovered
/// by scanning `c_src/build` rather than hard-coded.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("CB_C_SO") {
        return PathBuf::from(p);
    }
    let build = workspace_root().join("c_src/build");
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}\nBuild the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one lib*.so in {}, found {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

/// Every Rust `cdylib` present. Both the release and the debug artifact are
/// tested when both exist: the two are compiled at different optimisation
/// levels and can in principle differ, so parity must hold for each.
pub fn rust_so_paths() -> Vec<(String, PathBuf)> {
    if let Ok(p) = std::env::var("CB_RUST_SO") {
        return vec![("env".to_string(), PathBuf::from(p))];
    }
    let target = manifest_dir().join("target");
    let mut out = Vec::new();
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libcolourblind_lib.so");
        if p.is_file() {
            out.push((profile.to_string(), p));
        }
    }
    assert!(
        !out.is_empty(),
        "no Rust cdylib found under {}; run `cargo build --release` first",
        target.display()
    );
    out
}

// ---------------------------------------------------------------------------
// Loaded libraries
// ---------------------------------------------------------------------------

/// A loaded `.so` plus its resolved `colourblind` entry point.
pub struct Impl {
    pub label: String,
    pub path: PathBuf,
    pub call: CbFn,
    _lib: &'static Library,
}

impl Impl {
    fn open(label: &str, path: &Path) -> Impl {
        let lib: &'static Library = Box::leak(Box::new(unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()))
        }));
        let sym: Symbol<CbFn> = unsafe {
            lib.get(b"colourblind\0")
                .unwrap_or_else(|e| panic!("dlsym colourblind in {}: {e}", path.display()))
        };
        Impl {
            label: label.to_string(),
            path: path.to_path_buf(),
            call: *sym,
            _lib: lib,
        }
    }

    /// Invoke through the FFI boundary on three distinct floats.
    pub fn apply(&self, imp: c_int, rgb: [f32; 3]) -> [f32; 3] {
        // Three genuinely separate stack slots, so the pointers cannot alias.
        let mut rr = rgb[0];
        let mut gg = rgb[1];
        let mut bb = rgb[2];
        unsafe { (self.call)(imp, &mut rr, &mut gg, &mut bb) };
        [rr, gg, bb]
    }

    /// Invoke with a caller-chosen aliasing pattern. `slots` is the backing
    /// storage; `idx` selects which slot each of the three pointers refers to.
    pub fn apply_aliased(&self, imp: c_int, slots: &mut [f32; 3], idx: [usize; 3]) {
        let base = slots.as_mut_ptr();
        unsafe {
            (self.call)(
                imp,
                base.add(idx[0]),
                base.add(idx[1]),
                base.add(idx[2]),
            )
        };
    }
}

/// The C implementation (loaded once per test process).
pub fn c_impl() -> &'static Impl {
    static C: OnceLock<Impl> = OnceLock::new();
    C.get_or_init(|| Impl::open("c", &c_so_path()))
}

/// Every Rust implementation under test (loaded once per test process).
pub fn rust_impls() -> &'static [Impl] {
    static R: OnceLock<Vec<Impl>> = OnceLock::new();
    R.get_or_init(|| {
        rust_so_paths()
            .iter()
            .map(|(label, p)| Impl::open(&format!("rust-{label}"), p))
            .collect()
    })
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
// ---------------------------------------------------------------------------

pub fn bits(v: [f32; 3]) -> [u32; 3] {
    [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()]
}

pub fn show(v: [f32; 3]) -> String {
    format!(
        "[{:#010x} ({}), {:#010x} ({}), {:#010x} ({})]",
        v[0].to_bits(),
        v[0],
        v[1].to_bits(),
        v[1],
        v[2].to_bits(),
        v[2]
    )
}

/// Assert byte-identical results for one call, for every Rust `.so`.
/// Returns the number of comparisons performed, so callers can prove their
/// loops were not empty.
#[track_caller]
pub fn assert_same(row: &str, imp: c_int, input: [f32; 3]) -> u64 {
    let expect = c_impl().apply(imp, input);
    let mut n = 0;
    for r in rust_impls() {
        let got = r.apply(imp, input);
        n += 1;
        assert_eq!(
            bits(got),
            bits(expect),
            "\n[{row}] divergence in {} ({})\n  impairment : {} ({imp})\n  input      : {}\n  C   output : {}\n  Rust output: {}\n",
            r.label,
            r.path.display(),
            impairment_name(imp),
            show(input),
            show(expect),
            show(got),
        );
    }
    assert!(n > 0, "no Rust .so under test — [{row}] would be vacuous");
    n
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C01D_1234_5678;

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
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
    pub fn sign_bit(&mut self) -> u32 {
        (self.next_u32() & 1) << 31
    }
}

// ---------------------------------------------------------------------------
// Value classes (CONFIGS.md Axis 3)
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VClass {
    /// V1 — a typical colour channel in `[0, 1]`.
    InGamut,
    /// V2 — an arbitrary finite normal over the whole exponent range.
    FiniteNormal,
    /// V3 — `+0.0` / `-0.0`.
    SignedZero,
    /// V4 — a subnormal.
    Subnormal,
    /// V5 — `±FLT_MAX`, `±FLT_MIN` and neighbours: overflow / underflow feeders.
    Extreme,
    /// V6 — `±inf`.
    Infinity,
    /// V7 — a quiet NaN with a random sign and payload.
    QuietNan,
    /// V8 — a signalling NaN with a random sign and payload.
    SignallingNan,
    /// Any of the above, chosen per draw.
    Any,
}

pub const SPECIAL_CLASSES: [VClass; 6] = [
    VClass::SignedZero,
    VClass::Subnormal,
    VClass::Extreme,
    VClass::Infinity,
    VClass::QuietNan,
    VClass::SignallingNan,
];

pub const ALL_CLASSES: [VClass; 8] = [
    VClass::InGamut,
    VClass::FiniteNormal,
    VClass::SignedZero,
    VClass::Subnormal,
    VClass::Extreme,
    VClass::Infinity,
    VClass::QuietNan,
    VClass::SignallingNan,
];

const EXTREMES: [f32; 12] = [
    f32::MAX,
    -f32::MAX,
    f32::MIN_POSITIVE,
    -f32::MIN_POSITIVE,
    1.0e38,
    -1.0e38,
    3.4e38,
    -3.4e38,
    1.0e-38,
    -1.0e-38,
    f32::EPSILON,
    -f32::EPSILON,
];

pub fn draw(rng: &mut Rng, class: VClass) -> f32 {
    match class {
        VClass::InGamut => match rng.below(16) {
            0 => 0.0,
            1 => 1.0,
            2 => -0.0,
            _ => (rng.next_u32() >> 8) as f32 / ((1u32 << 24) as f32),
        },
        VClass::FiniteNormal => {
            let sign = rng.sign_bit();
            let exp = 1 + rng.below(254);
            let mant = rng.next_u32() & 0x007F_FFFF;
            f32::from_bits(sign | (exp << 23) | mant)
        }
        VClass::SignedZero => {
            if rng.next_u32() & 1 == 0 {
                0.0
            } else {
                -0.0
            }
        }
        VClass::Subnormal => {
            let sign = rng.sign_bit();
            let mut mant = rng.next_u32() & 0x007F_FFFF;
            if mant == 0 {
                mant = 1;
            }
            // Bias towards the very smallest subnormals too.
            if rng.next_u32() & 3 == 0 {
                mant = 1 + rng.below(64);
            }
            f32::from_bits(sign | mant)
        }
        VClass::Extreme => {
            let base = EXTREMES[rng.below(EXTREMES.len() as u32) as usize];
            // Perturb by a few ULPs so we are not always on the exact constant.
            let d = rng.below(5) as i32 - 2;
            f32::from_bits((base.to_bits() as i32).wrapping_add(d) as u32)
        }
        VClass::Infinity => {
            if rng.next_u32() & 1 == 0 {
                f32::INFINITY
            } else {
                f32::NEG_INFINITY
            }
        }
        VClass::QuietNan => {
            let sign = rng.sign_bit();
            let payload = rng.next_u32() & 0x003F_FFFF;
            f32::from_bits(sign | 0x7F80_0000 | 0x0040_0000 | payload)
        }
        VClass::SignallingNan => {
            let sign = rng.sign_bit();
            let mut payload = rng.next_u32() & 0x003F_FFFF;
            if payload == 0 {
                payload = 1;
            }
            f32::from_bits(sign | 0x7F80_0000 | payload)
        }
        VClass::Any => {
            let c = ALL_CLASSES[rng.below(ALL_CLASSES.len() as u32) as usize];
            draw(rng, c)
        }
    }
}

pub fn draw_triple(rng: &mut Rng, class: VClass) -> [f32; 3] {
    [draw(rng, class), draw(rng, class), draw(rng, class)]
}

/// V9 — exactly one channel is special, the other two are ordinary normals.
/// This is the shape that pins SSE's `dst`-before-`src` NaN priority per
/// instruction; a uniformly-special triple cannot distinguish the two orders.
pub fn draw_one_special(rng: &mut Rng) -> [f32; 3] {
    let pos = rng.below(3) as usize;
    let sclass = SPECIAL_CLASSES[rng.below(SPECIAL_CLASSES.len() as u32) as usize];
    let mut v = [
        draw(rng, VClass::FiniteNormal),
        draw(rng, VClass::FiniteNormal),
        draw(rng, VClass::FiniteNormal),
    ];
    v[pos] = draw(rng, sclass);
    v
}
