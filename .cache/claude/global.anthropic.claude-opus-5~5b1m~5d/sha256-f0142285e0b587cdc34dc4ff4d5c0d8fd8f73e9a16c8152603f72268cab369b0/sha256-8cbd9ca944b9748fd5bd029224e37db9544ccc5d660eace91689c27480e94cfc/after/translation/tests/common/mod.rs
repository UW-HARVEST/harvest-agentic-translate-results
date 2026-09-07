//! Shared harness: loads BOTH the C `.so` and the Rust `.so` via `libloading`
//! and calls `ldexp_q2` through the FFI boundary in both.
//!
//! Nothing here calls a Rust function of the crate directly — the Rust side is
//! always reached through `dlsym`, exactly as an external C consumer would, so
//! the `#[no_mangle]` / `extern "C"` export wrapper is under test too.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

pub type LdexpQ2 = unsafe extern "C" fn(f32, i32) -> f32;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("crate must live under the repo root")
        .to_path_buf()
}

/// Find the single `lib*.so` produced by the CMake build of `c_src`.
fn find_c_so() -> PathBuf {
    let build_dir = repo_root().join("c_src").join("build");
    assert!(
        build_dir.is_dir(),
        "C build dir {} not found. Build it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build_dir.display()
    );

    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .expect("read c_src/build")
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
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one lib*.so in {}, got {:?}",
        build_dir.display(),
        candidates
    );
    candidates.pop().unwrap()
}

/// Find the Rust cdylib. Prefers the profile the tests were built with, but
/// accepts the other one so `cargo test` works whether or not `--release` was
/// passed.
fn find_rust_so() -> PathBuf {
    let target = manifest_dir().join("target");
    let name = "libldexp_q2_lib.so";
    let preferred = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for profile in preferred {
        let p = target.join(profile).join(name);
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "Rust cdylib {} not found under {}. Build it with: cargo build --release",
        name,
        target.display()
    );
}

fn load(path: &Path) -> Library {
    unsafe { Library::new(path) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", path.display()))
}

struct Libs {
    c: Library,
    rust: Library,
}

// The libraries are kept alive for the whole test binary; the function pointers
// handed out below therefore stay valid.
static LIBS: OnceLock<Libs> = OnceLock::new();

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| Libs {
        c: load(&find_c_so()),
        rust: load(&find_rust_so()),
    })
}

fn sym(lib: &'static Library, which: &str) -> LdexpQ2 {
    let s: Symbol<'static, LdexpQ2> = unsafe { lib.get(b"ldexp_q2\0") }
        .unwrap_or_else(|e| panic!("{which} .so does not export `ldexp_q2`: {e}"));
    *s
}

/// `ldexp_q2` as exported by the **C** shared object.
pub fn c_ldexp_q2() -> LdexpQ2 {
    sym(&libs().c, "C")
}

/// `ldexp_q2` as exported by the **Rust** cdylib.
pub fn rust_ldexp_q2() -> LdexpQ2 {
    sym(&libs().rust, "Rust")
}

pub fn c_so_path() -> PathBuf {
    find_c_so()
}

pub fn rust_so_path() -> PathBuf {
    find_rust_so()
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible property testing.
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

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    /// Any `f32` bit pattern (all classes, including NaN payloads).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// A finite, normal `f32` with a wide exponent range.
    pub fn finite_normal_f32(&mut self) -> f32 {
        loop {
            let v = f32::from_bits(self.next_u32());
            if v.is_normal() {
                return v;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Differential assertion
// ---------------------------------------------------------------------------

/// Compare the two implementations bit-for-bit (`to_bits`), so `+0.0` vs `-0.0`
/// and differing NaN payloads are treated as divergences.
#[track_caller]
pub fn assert_same(row: &str, y: f32, exp_q2: i32) {
    let c = c_ldexp_q2();
    let r = rust_ldexp_q2();
    let cv = unsafe { c(y, exp_q2) };
    let rv = unsafe { r(y, exp_q2) };
    assert_eq!(
        cv.to_bits(),
        rv.to_bits(),
        "[{row}] divergence for ldexp_q2(y = {y:e} (bits 0x{:08x}), exp_q2 = {exp_q2}):\n  \
         C    = {cv:e} (bits 0x{:08x})\n  Rust = {rv:e} (bits 0x{:08x})",
        y.to_bits(),
        cv.to_bits(),
        rv.to_bits(),
    );
}

/// The representative `y` values covering every IEEE-754 class the C code can
/// see, plus saturation edges.
pub fn representative_ys() -> Vec<f32> {
    vec![
        0.0f32,
        -0.0f32,
        1.0,
        -1.0,
        2.0,
        -2.0,
        0.5,
        -0.5,
        3.0,
        7.0,
        1.0000001,
        0.9999999,
        std::f32::consts::PI,
        -std::f32::consts::E,
        1e10,
        -1e10,
        1e-10,
        -1e-10,
        1e38,
        -1e38,
        1e-38,
        -1e-38,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(0x0000_0001), // +FLT_TRUE_MIN (subnormal)
        f32::from_bits(0x8000_0001), // -FLT_TRUE_MIN
        f32::from_bits(0x007F_FFFF), // largest +subnormal
        f32::from_bits(0x807F_FFFF), // largest -subnormal
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_0001), // qNaN, non-default payload
        f32::from_bits(0xFFC0_0001), // negative qNaN, non-default payload
        f32::from_bits(0x7FA0_0000), // sNaN
        f32::from_bits(0xFFA0_0000), // negative sNaN
        f32::from_bits(0x7F80_0001), // sNaN, minimal payload
    ]
}

/// A representative `exp_q2` set: clamp boundaries, all four `e & 3` phases,
/// multi-iteration values, negatives (the UB shift path), and `int` extremes.
pub fn representative_exps() -> Vec<i32> {
    let mut v = vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 15, 16, 17, 63, 64, 116, 117, 118, 119, 120, 121, 122, 123, 124,
        125, 239, 240, 241, 360, 361, 1000, 4096,
        -1, -2, -3, -4, -5, -8, -16, -31, -32, -33, -64, -120, -121, -124, -128, -1000,
        i32::MIN, i32::MIN + 1, i32::MIN + 2, i32::MIN + 3, i32::MIN + 4,
        i32::MIN / 2, -(1 << 20), -(1 << 27),
    ];
    v.sort();
    v.dedup();
    v
}
