//! Shared differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` via `libloading` and exposes the
//! exported `memchra2` symbol from each. Nothing is ever called directly on the
//! Rust crate: every invocation goes through the dynamic-library boundary,
//! exactly as an external C consumer would.
//!
//! Each integration-test binary uses a different subset of this module, so
//! unused items are expected.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

pub type Memchra2 = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c_fn: Memchra2,
    rust_fn: Memchra2,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn c_so_path() -> PathBuf {
    // Allows the driver script to point the same differential suite at
    // alternative C builds (e.g. -O2) without touching c_src.
    if let Ok(p) = std::env::var("MEMCHRA2_C_SO") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    let root = workspace_root();
    let build = root.join("c_src").join("build");
    // The CMake project name is derived from the parent directory name, so the
    // library file name is not fixed. Discover it instead of hard-coding it.
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no lib*.so found in {}", build.display()))
}

fn rust_so_path() -> PathBuf {
    // The integration test binary lives in target/<profile>/deps/, so walk up
    // to the profile directory and look for the cdylib there.
    let mut dir = std::env::current_exe().expect("current_exe");
    dir.pop(); // test binary name
    if dir.ends_with("deps") {
        dir.pop();
    }
    for name in ["libmemchra2_lib.so"] {
        let p = dir.join(name);
        if p.exists() {
            return p;
        }
    }
    // Fall back to the release build, which is always produced by the driver
    // script before the tests run.
    let alt = workspace_root()
        .join("translation")
        .join("target")
        .join("release")
        .join("libmemchra2_lib.so");
    if alt.exists() {
        return alt;
    }
    panic!("libmemchra2_lib.so not found near {}", dir.display());
}

impl Pair {
    pub fn load() -> Pair {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("load C .so");
            let rust_lib = Library::new(rust_so_path()).expect("load Rust .so");
            let c_sym: Symbol<Memchra2> = c_lib.get(b"memchra2\0").expect("C memchra2");
            let rust_sym: Symbol<Memchra2> = rust_lib.get(b"memchra2\0").expect("Rust memchra2");
            let c_fn = *c_sym;
            let rust_fn = *rust_sym;
            Pair { _c_lib: c_lib, _rust_lib: rust_lib, c_fn, rust_fn }
        }
    }

    pub fn c(&self, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
        unsafe { (self.c_fn)(a, b, c, d) }
    }

    pub fn rust(&self, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
        unsafe { (self.rust_fn)(a, b, c, d) }
    }

    /// Asserts byte-identical results for one input tuple.
    #[track_caller]
    pub fn assert_same(&self, label: &str, a: c_int, b: c_int, c: c_int, d: c_int) {
        let got_c = self.c(a, b, c, d);
        let got_r = self.rust(a, b, c, d);
        assert_eq!(
            got_c, got_r,
            "[{label}] memchra2({a}, {b}, {c}, {d}) \
             (hex a=0x{a:08x} b=0x{b:08x} c=0x{c:08x} d=0x{d:08x}): C={got_c} Rust={got_r}"
        );
    }
}

/// Deterministic xorshift64* PRNG so every run is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `[lo, hi]` inclusive over u32.
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
}

/// The classes of `a` that the C code's `int_to_float_bits` branch
/// (`f > 0.0f && f < 1000.0f`) actually distinguishes. `a`'s object
/// representation is reinterpreted as an IEEE-754 binary32.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AClass {
    /// a == 0 -> f == +0.0 -> branch NOT taken
    Zero,
    /// a in [1, 0x007F_FFFF] -> positive subnormal, 0 < f < 2^-126 -> (int)f == 0
    PosSubnormal,
    /// a in [0x0080_0000, 0x3F7F_FFFF] -> 0 < f < 1.0 -> (int)f == 0
    PosNormLtOne,
    /// a in [0x3F80_0000, 0x4479_FFFF] -> 1.0 <= f < 1000.0 -> (int)f in [1, 999]
    PosNormInRange,
    /// a in [0x447A_0000, 0x7F7F_FFFF] -> f >= 1000.0 -> branch NOT taken
    PosGeThousand,
    /// a in [0x7F80_0000, 0x7FFF_FFFF] -> +inf / positive NaN -> branch NOT taken
    PosInfNan,
    /// a < 0 -> negative float / -inf / negative NaN -> branch NOT taken
    Negative,
}

pub const A_CLASSES: [AClass; 7] = [
    AClass::Zero,
    AClass::PosSubnormal,
    AClass::PosNormLtOne,
    AClass::PosNormInRange,
    AClass::PosGeThousand,
    AClass::PosInfNan,
    AClass::Negative,
];

impl AClass {
    pub fn name(self) -> &'static str {
        match self {
            AClass::Zero => "a=Zero",
            AClass::PosSubnormal => "a=PosSubnormal",
            AClass::PosNormLtOne => "a=PosNormLtOne",
            AClass::PosNormInRange => "a=PosNormInRange",
            AClass::PosGeThousand => "a=PosGeThousand",
            AClass::PosInfNan => "a=PosInfNan",
            AClass::Negative => "a=Negative",
        }
    }

    /// Draws a random `a` belonging to this class.
    pub fn sample(self, rng: &mut Rng) -> c_int {
        match self {
            AClass::Zero => 0,
            AClass::PosSubnormal => rng.range_u32(1, 0x007F_FFFF) as i32,
            AClass::PosNormLtOne => rng.range_u32(0x0080_0000, 0x3F7F_FFFF) as i32,
            AClass::PosNormInRange => rng.range_u32(0x3F80_0000, 0x4479_FFFF) as i32,
            AClass::PosGeThousand => rng.range_u32(0x447A_0000, 0x7F7F_FFFF) as i32,
            AClass::PosInfNan => rng.range_u32(0x7F80_0000, 0x7FFF_FFFF) as i32,
            AClass::Negative => (rng.range_u32(0x8000_0000, 0xFFFF_FFFF)) as i32,
        }
    }

    /// The inclusive boundary representatives of this class.
    pub fn boundaries(self) -> Vec<c_int> {
        match self {
            AClass::Zero => vec![0],
            AClass::PosSubnormal => vec![1, 2, 0x007F_FFFE, 0x007F_FFFF],
            AClass::PosNormLtOne => vec![0x0080_0000, 0x0080_0001, 0x3F7F_FFFE, 0x3F7F_FFFF],
            AClass::PosNormInRange => vec![
                0x3F80_0000, // 1.0
                0x3F80_0001,
                0x3FFF_FFFF, // just under 2.0
                0x4000_0000, // 2.0
                0x4479_FFFF, // just under 1000.0
                0x4479_FFFE,
            ],
            AClass::PosGeThousand => {
                vec![0x447A_0000, 0x447A_0001, 0x7F7F_FFFE, 0x7F7F_FFFF]
            }
            AClass::PosInfNan => vec![0x7F80_0000, 0x7F80_0001, 0x7FC0_0000, 0x7FFF_FFFF],
            AClass::Negative => vec![
                -1,          // 0xFFFFFFFF, negative NaN
                i32::MIN,    // 0x80000000, -0.0
                i32::MIN + 1,
                0xBF80_0000u32 as i32, // -1.0
                0xC479_FFFFu32 as i32,
                -2,
            ],
        }
    }
}

/// The eight sign patterns of `(b, c, d)`. The sign of each of these values
/// changes how many `-` characters `snprintf` writes into the buffer, which
/// feeds `count_occurrences(buffer, '-')` and `process_buffer`.
pub const BCD_SIGNS: [(bool, bool, bool); 8] = [
    (false, false, false),
    (false, false, true),
    (false, true, false),
    (false, true, true),
    (true, false, false),
    (true, false, true),
    (true, true, false),
    (true, true, true),
];

pub fn sign_label(s: (bool, bool, bool)) -> String {
    fn f(v: bool) -> char {
        if v {
            '-'
        } else {
            '+'
        }
    }
    format!("bcd={}{}{}", f(s.0), f(s.1), f(s.2))
}

/// Draws a value with the requested sign. `negative == false` yields
/// `[0, i32::MAX]`; `negative == true` yields `[i32::MIN, -1]`.
pub fn sample_signed(rng: &mut Rng, negative: bool) -> c_int {
    let mag = rng.next_u32();
    if negative {
        let v = (mag | 0x8000_0000) as i32;
        debug_assert!(v < 0);
        v
    } else {
        (mag & 0x7FFF_FFFF) as i32
    }
}

/// Values that stress the low byte extraction (`x & 0xFF`) used by
/// `interpret_as_int` / `complex_iteration`, plus generic extremes.
pub const INTERESTING: [c_int; 21] = [
    0,
    1,
    -1,
    2,
    -2,
    127,
    128,
    255,
    256,
    -127,
    -128,
    -255,
    -256,
    0x0000_00FF,
    0x0000_FF00,
    0x00FF_0000,
    0x7F00_0000,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
];

/// Number of randomized inputs used per `CONFIGS.md` row.
pub const ITERS_PER_ROW: usize = 400;
