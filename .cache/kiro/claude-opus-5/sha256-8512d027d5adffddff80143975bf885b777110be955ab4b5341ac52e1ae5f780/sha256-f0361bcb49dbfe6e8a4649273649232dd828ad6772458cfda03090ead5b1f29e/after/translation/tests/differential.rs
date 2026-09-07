//! Differential tests: C `.so` vs Rust `.so`, both loaded via `libloading`.
//!
//! Neither side is called as a Rust function. Both are resolved as `half2float`
//! dynamic symbols, exactly as an external C consumer would, so the
//! `#[unsafe(no_mangle)] extern "C"` export wrapper is under test too.
//!
//! Phase B rows live in `CONFIGS.md`; Phase C boundaries live in `ERRORS.md`.

use libloading::{Library, Symbol};
use std::path::PathBuf;

/// `float half2float(uint16_t)` — the real ABI of the exported symbol.
type Half2Float = unsafe extern "C" fn(u16) -> f32;

/// Same symbol viewed through a *wider* parameter slot, to probe narrow-argument
/// truncation across the FFI boundary (ERRORS.md row B4 / CONFIGS.md row 21).
type Half2FloatWide = unsafe extern "C" fn(u32) -> f32;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_library_path() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e}. Build the C library first:\n  cd c_src && mkdir -p build \
                 && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build.display()
            )
        })
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "so"))
        .collect();
    candidates.sort();
    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn rust_library_path() -> PathBuf {
    // The integration test binary lives in target/<profile>/deps/, so the cdylib
    // sits two levels up. Fall back to scanning both common profile dirs.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>");
    let name = "libhalf2float_lib.so";
    let direct = profile_dir.join(name);
    if direct.exists() {
        return direct;
    }
    for p in ["release", "debug"] {
        let alt = workspace_root()
            .join("translation")
            .join("target")
            .join(p)
            .join(name);
        if alt.exists() {
            return alt;
        }
    }
    panic!(
        "could not locate {name}; expected near {}. Run `cargo build` first.",
        profile_dir.display()
    );
}

/// Both libraries, kept alive for the duration of a test.
struct Pair {
    _c: Library,
    _rust: Library,
    c: Half2Float,
    rust: Half2Float,
    c_wide: Half2FloatWide,
    rust_wide: Half2FloatWide,
}

impl Pair {
    fn load() -> Self {
        unsafe {
            let c_lib = Library::new(c_library_path()).expect("load C .so");
            let rust_lib = Library::new(rust_library_path()).expect("load Rust .so");

            let c_sym: Symbol<Half2Float> =
                c_lib.get(b"half2float\0").expect("C exports half2float");
            let rust_sym: Symbol<Half2Float> = rust_lib
                .get(b"half2float\0")
                .expect("Rust .so exports half2float");
            let c_wide_sym: Symbol<Half2FloatWide> = c_lib.get(b"half2float\0").unwrap();
            let rust_wide_sym: Symbol<Half2FloatWide> = rust_lib.get(b"half2float\0").unwrap();

            let (c, rust) = (*c_sym, *rust_sym);
            let (c_wide, rust_wide) = (*c_wide_sym, *rust_wide_sym);

            Pair {
                _c: c_lib,
                _rust: rust_lib,
                c,
                rust,
                c_wide,
                rust_wide,
            }
        }
    }

    /// Compare **raw bits**, never `==`: `NaN != NaN` and `+0.0 == -0.0`, so a
    /// float comparison would silently accept two real divergence classes that
    /// this library actually produces (rows 1/9 and 8/16 of CONFIGS.md).
    #[track_caller]
    fn assert_same(&self, h: u16, row: &str) {
        let c = unsafe { (self.c)(h) };
        let r = unsafe { (self.rust)(h) };
        let (cb, rb) = (c.to_bits(), r.to_bits());
        assert_eq!(
            cb, rb,
            "{row}: divergence at h=0x{h:04x} (n={}, m=0x{:03x}): \
             C=0x{cb:08x} ({c:?})  Rust=0x{rb:08x} ({r:?})",
            h >> 10,
            h & 0x3ff,
        );
    }
}

/// splitmix64 — deterministic, fixed seed, so every failure is reproducible.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `[lo, hi]` inclusive.
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }
}

const SEED: u64 = 0x243F_6A88_85A3_08D3;

/// Build `h` from the two axes the C branches on.
fn h_of(n: u32, m: u32) -> u16 {
    debug_assert!(n < 64 && m < 1024);
    ((n << 10) | m) as u16
}

/// Randomized driver for one CONFIGS.md row: many seeded inputs, `n` drawn from
/// `n_range`, `m` drawn from `m_range`.
fn row_random(
    pair: &Pair,
    row: &str,
    n_range: (u32, u32),
    m_range: (u32, u32),
    iterations: usize,
) {
    let mut rng = Rng::new(SEED ^ (row.len() as u64) << 32 ^ n_range.0 as u64);
    for _ in 0..iterations {
        let n = rng.range(n_range.0, n_range.1);
        let m = rng.range(m_range.0, m_range.1);
        pair.assert_same(h_of(n, m), row);
    }
    // Always pin the edges of the row's own sub-domain too.
    for &n in &[n_range.0, n_range.1] {
        for &m in &[m_range.0, m_range.1] {
            pair.assert_same(h_of(n, m), row);
        }
    }
}

// ---------------------------------------------------------------------------
// Sanity: both symbols actually resolve through the dynamic loader.
// ---------------------------------------------------------------------------

#[test]
fn test_both_libraries_export_half2float() {
    let pair = Pair::load();
    // A resolved symbol that also runs is the real proof of an export wrapper.
    let c = unsafe { (pair.c)(0x3C00) };
    let r = unsafe { (pair.rust)(0x3C00) };
    assert_eq!(c.to_bits(), r.to_bits());
    assert_eq!(c, 1.0, "0x3C00 is half 1.0; C must yield exactly 1.0");
}

// ---------------------------------------------------------------------------
// Phase B — CONFIGS.md rows 1..21
// ---------------------------------------------------------------------------

#[test]
fn test_row01_positive_zero() {
    let pair = Pair::load();
    pair.assert_same(0x0000, "row 1: n=0 m=0 -> +0.0");
    let c = unsafe { (pair.c)(0x0000) };
    assert_eq!(c.to_bits(), 0x0000_0000, "row 1: C must give +0.0 bits");
}

#[test]
fn test_row02_positive_subnormal_half() {
    let pair = Pair::load();
    row_random(&Pair::load(), "row 2: n=0, m in [1,1023]", (0, 0), (1, 1023), 2000);
    // every single value in this row's domain (only 1023 of them)
    for m in 1..=1023 {
        pair.assert_same(h_of(0, m), "row 2 exhaustive");
    }
}

#[test]
fn test_row03_positive_normal_below_bias() {
    let pair = Pair::load();
    row_random(&pair, "row 3: n in [1,14]", (1, 14), (0, 1023), 5000);
}

#[test]
fn test_row04_positive_around_one() {
    let pair = Pair::load();
    row_random(&pair, "row 4: n=15", (15, 15), (0, 1023), 2000);
    for m in 0..1024 {
        pair.assert_same(h_of(15, m), "row 4 exhaustive");
    }
}

#[test]
fn test_row05_positive_normal_above_bias() {
    let pair = Pair::load();
    row_random(&pair, "row 5: n in [16,29]", (16, 29), (0, 1023), 5000);
}

#[test]
fn test_row06_positive_max_finite_exponent() {
    let pair = Pair::load();
    row_random(&pair, "row 6: n=30", (30, 30), (0, 1023), 2000);
    for m in 0..1024 {
        pair.assert_same(h_of(30, m), "row 6 exhaustive");
    }
}

#[test]
fn test_row07_positive_infinity() {
    let pair = Pair::load();
    pair.assert_same(0x7C00, "row 7: n=31 m=0 -> +inf");
    let c = unsafe { (pair.c)(0x7C00) };
    assert!(c.is_infinite() && c.is_sign_positive(), "row 7: C gives +inf");
}

#[test]
fn test_row08_positive_nan_payloads() {
    let pair = Pair::load();
    // NaN payload must survive bit-exactly; `assert_same` compares to_bits.
    for m in 1..=1023 {
        pair.assert_same(h_of(31, m), "row 8 exhaustive NaN payloads");
    }
    row_random(&pair, "row 8: n=31, m in [1,1023]", (31, 31), (1, 1023), 2000);
    let c = unsafe { (pair.c)(0x7C01) };
    assert!(c.is_nan(), "row 8: C gives NaN");
}

#[test]
fn test_row09_negative_zero() {
    let pair = Pair::load();
    pair.assert_same(0x8000, "row 9: n=32 m=0 -> -0.0");
    let c = unsafe { (pair.c)(0x8000) };
    assert_eq!(
        c.to_bits(),
        0x8000_0000,
        "row 9: C must give -0.0 bits, distinct from +0.0"
    );
}

#[test]
fn test_row10_negative_subnormal_half() {
    let pair = Pair::load();
    for m in 1..=1023 {
        pair.assert_same(h_of(32, m), "row 10 exhaustive");
    }
    row_random(&pair, "row 10: n=32, m in [1,1023]", (32, 32), (1, 1023), 2000);
}

#[test]
fn test_row11_negative_normal_below_bias() {
    let pair = Pair::load();
    row_random(&pair, "row 11: n in [33,46]", (33, 46), (0, 1023), 5000);
}

#[test]
fn test_row12_negative_around_minus_one() {
    let pair = Pair::load();
    row_random(&pair, "row 12: n=47", (47, 47), (0, 1023), 2000);
    for m in 0..1024 {
        pair.assert_same(h_of(47, m), "row 12 exhaustive");
    }
}

#[test]
fn test_row13_negative_normal_above_bias() {
    let pair = Pair::load();
    row_random(&pair, "row 13: n in [48,61]", (48, 61), (0, 1023), 5000);
}

#[test]
fn test_row14_negative_max_finite_exponent() {
    let pair = Pair::load();
    row_random(&pair, "row 14: n=62", (62, 62), (0, 1023), 2000);
    for m in 0..1024 {
        pair.assert_same(h_of(62, m), "row 14 exhaustive");
    }
}

#[test]
fn test_row15_negative_infinity() {
    let pair = Pair::load();
    pair.assert_same(0xFC00, "row 15: n=63 m=0 -> -inf");
    let c = unsafe { (pair.c)(0xFC00) };
    assert!(c.is_infinite() && c.is_sign_negative(), "row 15: C gives -inf");
}

#[test]
fn test_row16_negative_nan_payloads() {
    let pair = Pair::load();
    // Largest sums in the whole domain (exponent 0xc7800000) live here; this is
    // where a non-wrapping add would misbehave.
    for m in 1..=1023 {
        pair.assert_same(h_of(63, m), "row 16 exhaustive NaN payloads");
    }
    row_random(&pair, "row 16: n=63, m in [1,1023]", (63, 63), (1, 1023), 2000);
}

#[test]
fn test_row17_low_mantissa_region_rows() {
    let pair = Pair::load();
    // m__offset[n] == 0 exactly for n in {0, 32}: mantissa index stays in [0,1023].
    for n in [0u32, 32] {
        for m in [0u32, 1, 512, 1023] {
            pair.assert_same(h_of(n, m), "row 17: offset==0x0000 rows");
        }
        row_random(&pair, "row 17 randomized", (n, n), (0, 1023), 1500);
    }
}

#[test]
fn test_row18_high_mantissa_region_rows() {
    let pair = Pair::load();
    // m__offset[n] == 0x400 for the other 62 rows: index spans [1024, 2047].
    for n in 0..64u32 {
        if n == 0 || n == 32 {
            continue;
        }
        for m in [0u32, 1, 512, 1023] {
            pair.assert_same(h_of(n, m), "row 18: offset==0x0400 rows");
        }
    }
    row_random(&pair, "row 18 randomized lower half", (1, 31), (0, 1023), 3000);
    row_random(&pair, "row 18 randomized upper half", (33, 63), (0, 1023), 3000);
}

#[test]
fn test_row19_exhaustive_all_inputs() {
    let pair = Pair::load();
    // The entire domain is only 65_536 values, so "differential testing" can be
    // total here: this alone proves bit-exact equivalence for every input.
    let mut checked = 0u32;
    for h in 0..=u16::MAX {
        let c = unsafe { (pair.c)(h) };
        let r = unsafe { (pair.rust)(h) };
        assert_eq!(
            c.to_bits(),
            r.to_bits(),
            "row 19: divergence at h=0x{h:04x} (n={}, m=0x{:03x}): C=0x{:08x} Rust=0x{:08x}",
            h >> 10,
            h & 0x3ff,
            c.to_bits(),
            r.to_bits(),
        );
        checked += 1;
    }
    assert_eq!(checked, 65_536, "row 19 must cover the whole u16 domain");
}

#[test]
fn test_row20_randomized_stateless_sweep() {
    let pair = Pair::load();
    let mut rng = Rng::new(SEED);
    // Interleave C and Rust calls in random order to catch any hidden state.
    for _ in 0..200_000 {
        let h = rng.range(0, 0xFFFF) as u16;
        if rng.next_u64() & 1 == 0 {
            let c = unsafe { (pair.c)(h) };
            let r = unsafe { (pair.rust)(h) };
            assert_eq!(c.to_bits(), r.to_bits(), "row 20 at h=0x{h:04x}");
        } else {
            let r = unsafe { (pair.rust)(h) };
            let c = unsafe { (pair.c)(h) };
            assert_eq!(c.to_bits(), r.to_bits(), "row 20 (reversed) at h=0x{h:04x}");
        }
    }
    // Repeat-call determinism on both sides.
    for h in [0x0000u16, 0x3C00, 0x7C00, 0x7E00, 0x8000, 0xFC00, 0xFFFF] {
        let c0 = unsafe { (pair.c)(h) }.to_bits();
        let r0 = unsafe { (pair.rust)(h) }.to_bits();
        for _ in 0..64 {
            assert_eq!(unsafe { (pair.c)(h) }.to_bits(), c0, "C not stateless");
            assert_eq!(unsafe { (pair.rust)(h) }.to_bits(), r0, "Rust not stateless");
        }
    }
}

// ---------------------------------------------------------------------------
// Phase C — ERRORS.md boundaries B1..B6
// ---------------------------------------------------------------------------

#[test]
fn test_boundary_values() {
    let pair = Pair::load();
    // B1/B2/B3: domain minimum, maximum, and one step past every sub-range edge.
    let boundaries: &[u16] = &[
        0x0000, 0x0001, // min, min+1
        0x03FE, 0x03FF, 0x0400, 0x0401, // largest subnormal -> smallest normal
        0x7BFE, 0x7BFF, // largest positive finite
        0x7C00, 0x7C01, // +inf, first +NaN
        0x7FFE, 0x7FFF, // last positive NaN
        0x8000, 0x8001, // -0.0, first negative subnormal
        0x83FF, 0x8400, 0x8401, // negative subnormal -> negative normal
        0xFBFE, 0xFBFF, // largest negative finite
        0xFC00, 0xFC01, // -inf, first -NaN
        0xFFFE, 0xFFFF, // max, max-1
    ];
    for &h in boundaries {
        pair.assert_same(h, "ERRORS.md B1-B3 boundary");
    }
}

#[test]
fn test_no_error_channel_every_input_returns_a_value() {
    // ERRORS.md has zero rows: half2float is total, with no sentinel and no
    // error code. Verify that claim rather than trusting it -- every input must
    // return, and C and Rust must agree on the result for all of them.
    let pair = Pair::load();
    let mut nan = 0u32;
    let mut inf = 0u32;
    let mut finite = 0u32;
    for h in 0..=u16::MAX {
        let c = unsafe { (pair.c)(h) };
        let r = unsafe { (pair.rust)(h) };
        assert_eq!(c.to_bits(), r.to_bits(), "divergence at h=0x{h:04x}");
        // Classification must also agree, not just the bits.
        assert_eq!(c.is_nan(), r.is_nan(), "NaN-ness differs at h=0x{h:04x}");
        assert_eq!(
            c.is_infinite(),
            r.is_infinite(),
            "infinity differs at h=0x{h:04x}"
        );
        assert_eq!(
            c.is_sign_negative(),
            r.is_sign_negative(),
            "sign differs at h=0x{h:04x}"
        );
        if c.is_nan() {
            nan += 1;
        } else if c.is_infinite() {
            inf += 1;
        } else {
            finite += 1;
        }
    }
    // 2 * 1023 NaN encodings, 2 infinities, the rest finite.
    assert_eq!(nan, 2046, "expected 2046 NaN encodings");
    assert_eq!(inf, 2, "expected exactly +inf and -inf");
    assert_eq!(finite, 65_536 - 2046 - 2);
}

#[test]
fn test_ffi_dirty_upper_bits() {
    // ERRORS.md B4 / CONFIGS.md row 21: a C function with a `uint16_t` parameter
    // accepts any int at the ABI level -- the high bits of the argument register
    // are simply not part of the value. Passing a value with no valid u16
    // representation is therefore a real input that both sides must handle
    // identically. If the Rust export failed to truncate the way C's callee
    // does, this is where it would show up.
    let pair = Pair::load();
    let dirty_high: &[u32] = &[
        0x0000_0000,
        0x0001_0000,
        0xDEAD_0000,
        0xFFFF_0000,
        0x1234_0000,
        0x8000_0000,
    ];
    for &high in dirty_high {
        for low in [
            0x0000u32, 0x0001, 0x03FF, 0x0400, 0x3C00, 0x7C00, 0x7C01, 0x8000, 0xFC00, 0xFFFF,
        ] {
            let arg = high | low;
            let c = unsafe { (pair.c_wide)(arg) };
            let r = unsafe { (pair.rust_wide)(arg) };
            assert_eq!(
                c.to_bits(),
                r.to_bits(),
                "B4: divergence for wide arg 0x{arg:08x}: C=0x{:08x} Rust=0x{:08x}",
                c.to_bits(),
                r.to_bits(),
            );
        }
    }
    // Randomized dirty-bit fuzzing over the whole 32-bit slot.
    let mut rng = Rng::new(SEED ^ 0xB4B4_B4B4);
    for _ in 0..20_000 {
        let arg = rng.next_u64() as u32;
        let c = unsafe { (pair.c_wide)(arg) };
        let r = unsafe { (pair.rust_wide)(arg) };
        assert_eq!(
            c.to_bits(),
            r.to_bits(),
            "B4 fuzz: divergence for wide arg 0x{arg:08x}"
        );
    }
}

#[test]
fn test_table_index_extremes() {
    // The two provably-in-bounds index computations the C relies on without any
    // range check: mantissa index 0, 1023, 1024 and 2047, and exponent index
    // 0 and 63. Hitting each extreme directly guards against an off-by-one in
    // the Rust indexing or a mis-sized table.
    let pair = Pair::load();
    let extremes: &[(u32, u32, &str)] = &[
        (0, 0, "mantissa idx 0, exponent idx 0"),
        (0, 1023, "mantissa idx 1023 (low region top)"),
        (1, 0, "mantissa idx 1024 (high region base)"),
        (1, 1023, "mantissa idx 2047 (high region top)"),
        (31, 0, "exponent idx 31 (special +)"),
        (32, 0, "exponent idx 32 (sign only)"),
        (32, 1023, "mantissa idx 1023 via n=32"),
        (63, 1023, "exponent idx 63 (special -) with max mantissa"),
    ];
    for &(n, m, what) in extremes {
        pair.assert_same(h_of(n, m), what);
    }
}
