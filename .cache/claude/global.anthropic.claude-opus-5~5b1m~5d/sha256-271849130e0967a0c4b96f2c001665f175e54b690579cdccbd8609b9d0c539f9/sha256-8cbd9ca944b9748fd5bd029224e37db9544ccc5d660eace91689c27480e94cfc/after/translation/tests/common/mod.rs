// Shared differential-test harness: loads BOTH the C `.so` and the Rust `.so`
// via `libloading` and exposes matched pairs of function pointers so that every
// assertion crosses the real FFI boundary (exercising the `#[no_mangle]`
// export wrappers, never the Rust functions directly).

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_double, c_int};
use std::path::PathBuf;
use std::sync::OnceLock;

pub type FnSafeDoubleToInt = unsafe extern "C" fn(c_double) -> c_int;
pub type FnProcessArrayReverse = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
pub type FnSwitchFallthrough = unsafe extern "C" fn(c_int, c_int) -> c_int;
pub type FnAllocateAndCompute = unsafe extern "C" fn(c_int, c_double) -> c_int;
pub type FnForeachSum = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
pub type FnFallcalc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub safe_double_to_int: FnSafeDoubleToInt,
    pub process_array_reverse: FnProcessArrayReverse,
    pub switch_fallthrough_calculator: FnSwitchFallthrough,
    pub allocate_and_compute: FnAllocateAndCompute,
    pub foreach_sum: FnForeachSum,
    pub fallcalc: FnFallcalc,
}

impl Impl {
    unsafe fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = unsafe {
            Library::new(path).unwrap_or_else(|e| panic!("failed to dlopen {path:?}: {e}"))
        };
        macro_rules! sym {
            ($t:ty, $n:literal) => {{
                let s: Symbol<$t> = unsafe {
                    lib.get($n)
                        .unwrap_or_else(|e| {
                            panic!(
                                "{} missing symbol {}: {}",
                                name,
                                String::from_utf8_lossy(&$n[..$n.len() - 1]),
                                e
                            )
                        })
                };
                unsafe { *s.into_raw() }
            }};
        }
        Impl {
            name,
            safe_double_to_int: sym!(FnSafeDoubleToInt, b"safe_double_to_int\0"),
            process_array_reverse: sym!(FnProcessArrayReverse, b"process_array_reverse\0"),
            switch_fallthrough_calculator: sym!(FnSwitchFallthrough, b"switch_fallthrough_calculator\0"),
            allocate_and_compute: sym!(FnAllocateAndCompute, b"allocate_and_compute\0"),
            foreach_sum: sym!(FnForeachSum, b"foreach_sum\0"),
            fallcalc: sym!(FnFallcalc, b"fallcalc\0"),
            _lib: lib,
        }
    }
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    let dir = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {dir:?} ({e}). Build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
            )
        })
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    candidates.sort();
    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no .so found in {dir:?}"))
}

fn find_rust_so() -> PathBuf {
    // Allow the CI script to pin a specific profile's artifact.
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_SO_PATH does not exist: {p:?}");
        return p;
    }
    // Prefer the profile the tests were built with, then fall back.
    let target = workspace_root().join("translation").join("target");
    let mut tried = Vec::new();
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libfallcalc_lib.so");
        if p.exists() {
            return p;
        }
        tried.push(p);
    }
    panic!("Rust cdylib not found; tried {tried:?}. Run `cargo build --release` first.");
}

static PAIR: OnceLock<Pair> = OnceLock::new();

pub fn pair() -> &'static Pair {
    PAIR.get_or_init(|| {
        let c_path = find_c_so();
        let r_path = find_rust_so();
        unsafe {
            Pair {
                c: Impl::load("C", &c_path),
                rust: Impl::load("Rust", &r_path),
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) so every property test is reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Uniform random bit pattern reinterpreted as f64 (NaN/Inf/subnormal included).
    pub fn next_f64_bits(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit() * (hi - lo)
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

/// Values that historically trip C/Rust divergences.
pub const INTERESTING: &[i32] = &[
    0, 1, -1, 2, -2, 3, -3, 4, -4, 5, -5, 6, 7, 8, 9, 10, -9, -10, -11, -21, 63, 64, 65, 127, 128,
    129, 255, 256, 511, 512, 1000, -1000, 65535, 65536, 1 << 20, -(1 << 20), 1 << 30, -(1 << 30),
    i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, i32::MAX / 2, i32::MIN / 2,
];

/// Doubles that exercise every branch of `safe_double_to_int`.
pub fn interesting_doubles() -> Vec<f64> {
    let imax = i32::MAX as f64;
    let imin = i32::MIN as f64;
    let mut v = vec![
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF0_0000_0000_0001), // sNaN
        f64::from_bits(0xFFF8_0000_0000_0000), // negative qNaN
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,
        -5e-324,
        f64::MAX,
        f64::MIN,
        1e300,
        -1e300,
        0.5,
        -0.5,
        0.9999999999,
        -0.9999999999,
        1.5,
        -1.5,
        -2.5,
        2.5,
        imax,
        imin,
        imax + 1.0,
        imin - 1.0,
        imax - 0.5,
        imin + 0.5,
        imax - 1.0,
        imin + 1.0,
    ];
    for base in [imax, imin] {
        let mut x = base;
        for _ in 0..4 {
            x = next_after(x, 0.0);
            v.push(x);
        }
        let mut y = base;
        for _ in 0..4 {
            y = next_after(y, if base > 0.0 { f64::INFINITY } else { f64::NEG_INFINITY });
            v.push(y);
        }
    }
    v
}

/// Minimal `nextafter` for finite inputs (no libm dependency).
pub fn next_after(x: f64, toward: f64) -> f64 {
    if x.is_nan() || toward.is_nan() {
        return f64::NAN;
    }
    if x == toward {
        return toward;
    }
    if x == 0.0 {
        return if toward > 0.0 { 5e-324 } else { -5e-324 };
    }
    let bits = x.to_bits();
    let up = (x < toward) == (x > 0.0);
    f64::from_bits(if up { bits + 1 } else { bits - 1 })
}

/// Assert both implementations agree; `ctx` names the configuration row.
#[track_caller]
pub fn eq(ctx: &str, c: c_int, r: c_int) {
    assert_eq!(
        c, r,
        "DIVERGENCE [{ctx}]: C returned {c} (0x{c:08x}), Rust returned {r} (0x{r:08x})"
    );
}
