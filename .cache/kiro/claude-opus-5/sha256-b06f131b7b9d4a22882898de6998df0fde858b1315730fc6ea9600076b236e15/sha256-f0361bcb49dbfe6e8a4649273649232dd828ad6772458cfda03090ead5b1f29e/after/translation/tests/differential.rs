//! Differential tests: C `.so` vs Rust `.so`, both loaded with `libloading`.
//!
//! Neither side is ever called directly from Rust code — every invocation goes
//! through `dlopen`/`dlsym` on the built shared objects, so the `#[no_mangle]`
//! `extern "C"` export wrapper of the Rust crate is under test too.
//!
//! * Phase B rows live in `CONFIGS.md` and are named `config_cNN_*`.
//! * Phase C rows live in `ERRORS.md` and are named `error_eN_*`.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

type DivEuclid = unsafe extern "C" fn(std::ffi::c_int, std::ffi::c_int) -> std::ffi::c_int;

struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c: DivEuclid,
    rust: DivEuclid,
}

// The raw function pointers are plain `extern "C" fn`s into two libraries that
// hold no mutable state, so sharing them across threads is sound. The
// `Library` handles are kept alive for the whole process lifetime.
unsafe impl Send for Pair {}
unsafe impl Sync for Pair {}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// The C `.so` name is derived by `CMakeLists.txt` from the *parent* directory
/// name, so glob the build directory instead of hard-coding it.
fn find_c_so() -> PathBuf {
    let build_dir = workspace_root().join("c_src/build");
    let mut hits: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| {
            panic!(
                "cannot read {} ({e}); build the C library first:\n  \
                 cd c_src && mkdir -p build && cd build && \
                 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
                build_dir.display()
            )
        })
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "so"))
        .collect();
    hits.sort();
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one .so in {}, found {hits:?}",
        build_dir.display()
    );
    hits.pop().unwrap()
}

fn find_rust_so() -> PathBuf {
    let target = workspace_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libdiv_euclid_lib.so");
        if p.is_file() {
            assert_not_stale(&p);
            return p;
        }
    }
    panic!(
        "libdiv_euclid_lib.so not found under {}; run `cargo build --release` first",
        target.display()
    );
}

/// `cargo test` does NOT rebuild a `cdylib`-only library (no test target can
/// link it), so the `.so` on disk can silently lag behind `src/lib.rs` and the
/// whole suite would then validate stale code. Refuse to run in that case.
fn assert_not_stale(so: &Path) {
    let src = workspace_root().join("translation/src/lib.rs");
    let so_t = std::fs::metadata(so).and_then(|m| m.modified());
    let src_t = std::fs::metadata(&src).and_then(|m| m.modified());
    if let (Ok(so_t), Ok(src_t)) = (so_t, src_t) {
        assert!(
            so_t >= src_t,
            "{} is OLDER than {} — `cargo test` does not rebuild a cdylib.\n\
             Run `cargo build --release` (or use ./run_all.sh) before `cargo test`.",
            so.display(),
            src.display()
        );
    }
}

fn libs() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| unsafe {
        let c_path = find_c_so();
        let rust_path = find_rust_so();
        eprintln!("C   .so: {}", c_path.display());
        eprintln!("Rust.so: {}", rust_path.display());

        let c_lib = Library::new(&c_path).expect("dlopen C .so");
        let rust_lib = Library::new(&rust_path).expect("dlopen Rust .so");

        let c_sym: Symbol<DivEuclid> = c_lib
            .get(b"div_euclid\0")
            .expect("C .so must export div_euclid");
        let rust_sym: Symbol<DivEuclid> = rust_lib
            .get(b"div_euclid\0")
            .expect("Rust .so must export div_euclid");

        let c = *c_sym;
        let rust = *rust_sym;
        Pair {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
        }
    })
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

/// Calls both shared objects and returns `(c_result, rust_result)`.
fn both(v1: i32, v2: i32) -> (i32, i32) {
    let p = libs();
    unsafe { ((p.c)(v1, v2), (p.rust)(v1, v2)) }
}

#[track_caller]
fn check(v1: i32, v2: i32) {
    let (c, r) = both(v1, v2);
    assert_eq!(
        c, r,
        "div_euclid({v1}, {v2}): C .so returned {c}, Rust .so returned {r}"
    );
}

#[track_caller]
fn check_all(cases: &[(i32, i32)]) {
    for &(v1, v2) in cases {
        check(v1, v2);
    }
}

/// Assert both sides agree AND that the shared value is the exact sentinel the
/// C source is documented (in `ERRORS.md`) to produce.
#[track_caller]
fn check_exact(v1: i32, v2: i32, expected: i32) {
    let (c, r) = both(v1, v2);
    assert_eq!(
        c, expected,
        "C .so div_euclid({v1}, {v2}) returned {c}, ERRORS.md says {expected}"
    );
    assert_eq!(
        r, c,
        "div_euclid({v1}, {v2}): C .so returned {c}, Rust .so returned {r}"
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed — property-style testing, reproducible)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // Avoid the all-zero state of xorshift64*.
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `[lo, hi]` inclusive (works across the full `i32` range).
    fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    /// Magnitude-biased: picks a random bit width first, so small values,
    /// mid-range values and near-`i32::MAX` values all appear often.
    fn biased_i32(&mut self) -> i32 {
        let bits = (self.next_u64() % 32) as u32 + 1;
        let mask = if bits >= 64 { u64::MAX } else { (1u64 << bits) - 1 };
        let mag = (self.next_u64() & mask) as i64;
        let v = if self.next_u64() & 1 == 0 { mag } else { -mag };
        // Fold into i32, keeping i32::MIN reachable.
        v as i32
    }
    fn nonzero_i32(&mut self) -> i32 {
        loop {
            let v = self.next_i32();
            if v != 0 {
                return v;
            }
        }
    }
    fn biased_nonzero_i32(&mut self) -> i32 {
        loop {
            let v = self.biased_i32();
            if v != 0 {
                return v;
            }
        }
    }
}

/// Values that sit on or next to every boundary the C source mentions.
const BOUNDARY: &[i32] = &[
    i32::MIN,
    i32::MIN + 1,
    i32::MIN + 2,
    -0x4000_0001,
    -0x4000_0000,
    -0x3FFF_FFFF,
    -65_537,
    -65_536,
    -65_535,
    -1_000_000,
    -257,
    -256,
    -255,
    -128,
    -127,
    -100,
    -17,
    -16,
    -15,
    -10,
    -7,
    -3,
    -2,
    -1,
    0,
    1,
    2,
    3,
    7,
    10,
    15,
    16,
    17,
    100,
    127,
    128,
    255,
    256,
    257,
    1_000_000,
    65_535,
    65_536,
    65_537,
    0x3FFF_FFFF,
    0x4000_0000,
    0x4000_0001,
    i32::MAX - 2,
    i32::MAX - 1,
    i32::MAX,
];

const SEED: u64 = 0x5EED_1234_ABCD_0001;
const RANDOM_CASES: usize = 20_000;

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

/// C01: `v2 == 0` for every dividend class and many random dividends.
#[test]
fn config_c01_divisor_zero_all_dividend_classes() {
    check_all(&[
        (0, 0),
        (1, 0),
        (-1, 0),
        (i32::MAX, 0),
        (i32::MIN, 0),
        (i32::MIN + 1, 0),
        (i32::MAX - 1, 0),
    ]);
    for &v1 in BOUNDARY {
        check(v1, 0);
    }
    let mut rng = Rng::new(SEED ^ 0x01);
    for _ in 0..RANDOM_CASES {
        check(rng.next_i32(), 0);
        check(rng.biased_i32(), 0);
    }
}

/// C02: `v1 >= 0`, `v2 > 0`, `v1 < v2` — quotient 0, early `return v1/v2`.
#[test]
fn config_c02_nonneg_over_positive_smaller_dividend() {
    check_all(&[(0, 1), (1, 2), (5, 7), (1, i32::MAX), (i32::MAX - 1, i32::MAX)]);
    let mut rng = Rng::new(SEED ^ 0x02);
    for _ in 0..RANDOM_CASES {
        let v2 = rng.range_i32(2, i32::MAX);
        let v1 = rng.range_i32(0, v2 - 1);
        check(v1, v2);
    }
}

/// C03: `v1 >= 0`, `v2 > 0`, `v1 == v2`.
#[test]
fn config_c03_nonneg_over_positive_equal() {
    check_all(&[(1, 1), (2, 2), (i32::MAX, i32::MAX), (65_536, 65_536)]);
    let mut rng = Rng::new(SEED ^ 0x03);
    for _ in 0..RANDOM_CASES {
        let v = rng.range_i32(1, i32::MAX);
        check(v, v);
    }
}

/// C04: `v1 >= 0`, `v2 > 0`, `v1 > v2`, exactly divisible.
#[test]
fn config_c04_nonneg_over_positive_exact() {
    check_all(&[(10, 5), (i32::MAX - 1, 2), (0x4000_0000, 2), (1_000_000, 1_000)]);
    let mut rng = Rng::new(SEED ^ 0x04);
    for _ in 0..RANDOM_CASES {
        let v2 = rng.range_i32(1, 46_340); // 46340^2 < i32::MAX
        let q = rng.range_i32(1, i32::MAX / v2);
        check(v2.wrapping_mul(q), v2);
    }
}

/// C05: `v1 >= 0`, `v2 > 0`, `v1 > v2`, not divisible.
#[test]
fn config_c05_nonneg_over_positive_inexact() {
    check_all(&[(11, 5), (i32::MAX, 2), (i32::MAX, 3), (7, 3)]);
    let mut rng = Rng::new(SEED ^ 0x05);
    for _ in 0..RANDOM_CASES {
        let v2 = rng.range_i32(2, 1 << 20);
        let v1 = rng.range_i32(v2 + 1, i32::MAX);
        check(v1, v2);
        check(v1, v2.wrapping_add(if v1 % v2 == 0 { 1 } else { 0 }).max(1));
    }
}

/// C06: `v1 == 0`, `v2 > 0`.
#[test]
fn config_c06_zero_over_positive() {
    check_all(&[(0, 1), (0, 2), (0, i32::MAX), (0, i32::MAX - 1)]);
    let mut rng = Rng::new(SEED ^ 0x06);
    for _ in 0..RANDOM_CASES {
        check(0, rng.range_i32(1, i32::MAX));
    }
}

/// C07: `v1 >= 0`, `INT_MIN < v2 < 0`, `|v1| < |v2|` → `q = 0`, `r = v1 >= 0`.
#[test]
fn config_c07_nonneg_over_negative_smaller_dividend() {
    check_all(&[(0, -1), (1, -2), (5, -7), (1, i32::MIN + 1), (i32::MAX - 1, i32::MIN + 1)]);
    let mut rng = Rng::new(SEED ^ 0x07);
    for _ in 0..RANDOM_CASES {
        let mag = rng.range_i32(2, i32::MAX);
        let v1 = rng.range_i32(0, mag - 1);
        check(v1, -mag);
    }
}

/// C08: `v1 >= 0`, `INT_MIN < v2 < 0`, exactly divisible.
#[test]
fn config_c08_nonneg_over_negative_exact() {
    check_all(&[(10, -5), (i32::MAX - 1, -2), (0x4000_0000, -2), (0, -3)]);
    let mut rng = Rng::new(SEED ^ 0x08);
    for _ in 0..RANDOM_CASES {
        let mag = rng.range_i32(1, 46_340);
        let q = rng.range_i32(0, i32::MAX / mag);
        check(mag.wrapping_mul(q), -mag);
    }
}

/// C09: `v1 >= 0`, `INT_MIN < v2 < 0`, not divisible (`r > 0`, still `return q`).
#[test]
fn config_c09_nonneg_over_negative_inexact() {
    check_all(&[(11, -5), (i32::MAX, -2), (i32::MAX, -3), (7, -3)]);
    let mut rng = Rng::new(SEED ^ 0x09);
    for _ in 0..RANDOM_CASES {
        let mag = rng.range_i32(2, i32::MAX);
        let v1 = rng.range_i32(0, i32::MAX);
        check(v1, -mag);
    }
}

/// C10: `v1 == 0`, `INT_MIN < v2 < 0`.
#[test]
fn config_c10_zero_over_negative() {
    check_all(&[(0, -1), (0, -2), (0, i32::MIN + 1), (0, i32::MAX * -1)]);
    let mut rng = Rng::new(SEED ^ 0x0A);
    for _ in 0..RANDOM_CASES {
        check(0, rng.range_i32(i32::MIN + 1, -1));
    }
}

/// C11: `v1 > 0`, `v2 == INT_MIN` → `q = 0, r = v1`.
#[test]
fn config_c11_positive_over_int_min() {
    check_all(&[(1, i32::MIN), (2, i32::MIN), (i32::MAX, i32::MIN), (i32::MAX - 1, i32::MIN)]);
    let mut rng = Rng::new(SEED ^ 0x0B);
    for _ in 0..RANDOM_CASES {
        check(rng.range_i32(1, i32::MAX), i32::MIN);
    }
}

/// C12: `v1 == 0`, `v2 == INT_MIN`.
#[test]
fn config_c12_zero_over_int_min() {
    check_exact(0, i32::MIN, 0);
}

/// C13: `INT_MIN < v1 < 0`, `v2 > 0`, exactly divisible → `return q`.
#[test]
fn config_c13_negative_over_positive_exact() {
    check_all(&[(-10, 5), (-1, 1), (i32::MIN + 1, 1), (-0x4000_0000, 2)]);
    let mut rng = Rng::new(SEED ^ 0x0D);
    for _ in 0..RANDOM_CASES {
        let v2 = rng.range_i32(1, 46_340);
        let q = rng.range_i32(1, i32::MAX / v2);
        check(-(v2.wrapping_mul(q)), v2);
    }
}

/// C14: `INT_MIN < v1 < 0`, `v2 > 0`, `r < 0` → `return q - 1`.
#[test]
fn config_c14_negative_over_positive_inexact() {
    check_all(&[(-11, 5), (-7, 3), (i32::MIN + 1, 2), (i32::MIN + 1, 3)]);
    let mut rng = Rng::new(SEED ^ 0x0E);
    for _ in 0..RANDOM_CASES {
        let v1 = rng.range_i32(i32::MIN + 1, -1);
        let v2 = rng.range_i32(1, i32::MAX);
        check(v1, v2);
    }
}

/// C15: `INT_MIN < v1 < 0`, `v2 > 0`, `|v1| < v2` → `q = 0`, then `-1`.
#[test]
fn config_c15_negative_over_larger_positive() {
    check_all(&[(-1, 2), (-5, 7), (i32::MIN + 1, i32::MAX), (-1, i32::MAX)]);
    let mut rng = Rng::new(SEED ^ 0x0F);
    for _ in 0..RANDOM_CASES {
        let v2 = rng.range_i32(2, i32::MAX);
        let v1 = -rng.range_i32(1, v2 - 1);
        check(v1, v2);
    }
}

/// C16: `INT_MIN < v1 < 0`, `INT_MIN < v2 < 0`, `r == 0` → `return q`.
#[test]
fn config_c16_negative_over_negative_exact() {
    check_all(&[(-10, -5), (-1, -1), (i32::MIN + 1, -1), (-0x4000_0000, -2)]);
    let mut rng = Rng::new(SEED ^ 0x10);
    for _ in 0..RANDOM_CASES {
        let mag = rng.range_i32(1, 46_340);
        let q = rng.range_i32(1, i32::MAX / mag);
        check(-(mag.wrapping_mul(q)), -mag);
    }
}

/// C17: `INT_MIN < v1 < 0`, `INT_MIN < v2 < 0`, `r < 0` → `return q + 1`.
#[test]
fn config_c17_negative_over_negative_inexact() {
    check_all(&[(-11, -5), (-7, -3), (i32::MIN + 1, -2), (i32::MIN + 1, -3)]);
    let mut rng = Rng::new(SEED ^ 0x11);
    for _ in 0..RANDOM_CASES {
        let v1 = rng.range_i32(i32::MIN + 1, -1);
        let v2 = rng.range_i32(i32::MIN + 1, -1);
        check(v1, v2);
    }
}

/// C18: `INT_MIN < v1 < 0`, `INT_MIN < v2 < 0`, `|v1| < |v2|` → `q = 0`, then `+1`.
#[test]
fn config_c18_negative_over_larger_negative() {
    check_all(&[(-1, -2), (-5, -7), (-1, i32::MIN + 1), (i32::MIN + 2, i32::MIN + 1)]);
    let mut rng = Rng::new(SEED ^ 0x12);
    for _ in 0..RANDOM_CASES {
        let mag = rng.range_i32(2, i32::MAX);
        let v1 = -rng.range_i32(1, mag - 1);
        check(v1, -mag);
    }
}

/// C19: `INT_MIN < v1 < 0`, `v2 == INT_MIN` → `q = 1, r = v1 - INT_MIN > 0`.
#[test]
fn config_c19_negative_over_int_min() {
    check_all(&[(-1, i32::MIN), (i32::MIN + 1, i32::MIN), (-65_536, i32::MIN)]);
    let mut rng = Rng::new(SEED ^ 0x13);
    for _ in 0..RANDOM_CASES {
        check(rng.range_i32(i32::MIN + 1, -1), i32::MIN);
    }
}

/// C20: `v1 == INT_MIN`, `v2 > 0`, `r == 0` (`v2` divides `INT_MIN`).
#[test]
fn config_c20_int_min_over_positive_exact() {
    // Only powers of two divide INT_MIN exactly.
    for k in 0..32u32 {
        let v2 = 1i32.wrapping_shl(k);
        if v2 > 0 {
            check(i32::MIN, v2);
        }
    }
    check_all(&[(i32::MIN, 1), (i32::MIN, 2), (i32::MIN, 0x4000_0000)]);
}

/// C21: `v1 == INT_MIN`, `v2 > 0`, `r < 0` → `return q - 1`.
#[test]
fn config_c21_int_min_over_positive_inexact() {
    check_all(&[(i32::MIN, 3), (i32::MIN, 5), (i32::MIN, 7), (i32::MIN, i32::MAX - 1)]);
    let mut rng = Rng::new(SEED ^ 0x15);
    for _ in 0..RANDOM_CASES {
        check(i32::MIN, rng.range_i32(1, i32::MAX));
        check(i32::MIN, rng.biased_i32().saturating_abs().max(1));
    }
}

/// C22: `v1 == INT_MIN`, `v2 == 1` — `-(v1 + v2) == INT_MAX`.
#[test]
fn config_c22_int_min_over_one() {
    check(i32::MIN, 1);
    check(i32::MIN, 2);
}

/// C23: `v1 == INT_MIN`, `v2 == INT_MAX` — `-(v1 + v2) == 1`.
#[test]
fn config_c23_int_min_over_int_max() {
    check(i32::MIN, i32::MAX);
    check(i32::MIN, i32::MAX - 1);
}

/// C24: `v1 == INT_MIN`, `INT_MIN < v2 < 0`, `r == 0`.
#[test]
fn config_c24_int_min_over_negative_exact() {
    for k in 0..31u32 {
        let v2 = -(1i32.wrapping_shl(k));
        check(i32::MIN, v2);
    }
    check_all(&[(i32::MIN, -1), (i32::MIN, -2), (i32::MIN, -0x4000_0000)]);
}

/// C25: `v1 == INT_MIN`, `INT_MIN < v2 < 0`, `r < 0` → `return q + 1`.
#[test]
fn config_c25_int_min_over_negative_inexact() {
    check_all(&[(i32::MIN, -3), (i32::MIN, -5), (i32::MIN, -7), (i32::MIN, i32::MIN + 2)]);
    let mut rng = Rng::new(SEED ^ 0x19);
    for _ in 0..RANDOM_CASES {
        check(i32::MIN, rng.range_i32(i32::MIN + 1, -1));
    }
}

/// C26: `v1 == INT_MIN`, `v2 == -1` — `-(v1 - v2) == INT_MAX`, quotient
/// overflows `int` inside the C expression.
#[test]
fn config_c26_int_min_over_minus_one() {
    check(i32::MIN, -1);
    check(i32::MIN, -2);
}

/// C27: `v1 == INT_MIN`, `v2 == INT_MIN + 1` — `-(v1 - v2) == 1`.
#[test]
fn config_c27_int_min_over_int_min_plus_one() {
    check(i32::MIN, i32::MIN + 1);
    check(i32::MIN, i32::MIN + 2);
    check(i32::MIN + 1, i32::MIN + 1);
}

/// C28: `v1 == INT_MIN`, `v2 == INT_MIN` → `q = 1, r = 0`.
#[test]
fn config_c28_int_min_over_int_min() {
    check_exact(i32::MIN, i32::MIN, 1);
}

/// C29: unrestricted uniform random over all of `i32 × i32`.
#[test]
fn config_c29_uniform_random_full_domain() {
    let mut rng = Rng::new(SEED ^ 0x1D);
    for _ in 0..500_000 {
        check(rng.next_i32(), rng.next_i32());
    }
}

/// C30: exhaustive dense neighbourhood `[-256, 256]²` (263 169 pairs).
#[test]
fn config_c30_exhaustive_dense_neighbourhood() {
    for v1 in -256..=256i32 {
        for v2 in -256..=256i32 {
            check(v1, v2);
        }
    }
}

/// C31: exhaustive cross-product of every boundary value with every other.
#[test]
fn config_c31_exhaustive_boundary_cross_product() {
    for &v1 in BOUNDARY {
        for &v2 in BOUNDARY {
            check(v1, v2);
        }
    }
    // Also every boundary against powers of two (and their neighbours), both signs.
    let mut pow2: Vec<i32> = Vec::new();
    for k in 0..31u32 {
        let p = 1i32 << k;
        pow2.extend_from_slice(&[p, -p]);
        if p > 1 {
            pow2.extend_from_slice(&[p - 1, p + 1, -(p - 1), -(p + 1)]);
        }
    }
    pow2.push(i32::MIN);
    for &v1 in BOUNDARY {
        for &v2 in &pow2 {
            check(v1, v2);
        }
    }
    for &v1 in &pow2 {
        for &v2 in &pow2 {
            check(v1, v2);
        }
    }
}

/// C32: `v1` at the positive extremes against every divisor class.
#[test]
fn config_c32_int_max_dividend_all_divisor_classes() {
    for v1 in [i32::MAX, i32::MAX - 1, 0x4000_0000, 0x4000_0001] {
        for &v2 in BOUNDARY {
            check(v1, v2);
        }
    }
    let mut rng = Rng::new(SEED ^ 0x20);
    for _ in 0..RANDOM_CASES {
        check(i32::MAX, rng.next_i32());
        check(i32::MAX - 1, rng.next_i32());
    }
}

/// C33: magnitude-biased random inputs (many near-extreme operands).
#[test]
fn config_c33_magnitude_biased_random() {
    let mut rng = Rng::new(SEED ^ 0x21);
    for _ in 0..500_000 {
        check(rng.biased_i32(), rng.biased_i32());
    }
    for _ in 0..100_000 {
        check(rng.biased_i32(), rng.biased_nonzero_i32());
        check(rng.nonzero_i32(), rng.biased_i32());
    }
}

/// C34 (stress): exhaustive sweep of EVERY one of the 2^32 possible divisors
/// against the hardest dividends. This is the strongest guarantee available
/// short of the full 2^64 domain: for these `v1` values every reachable
/// `(branch leaf, remainder sign, magnitude)` combination is visited.
#[test]
fn config_c34_exhaustive_divisor_sweep_for_critical_dividends() {
    let p = libs();
    for v1 in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX] {
        let mut v2: i32 = i32::MIN;
        loop {
            let (c, r) = unsafe { ((p.c)(v1, v2), (p.rust)(v1, v2)) };
            if c != r {
                panic!("div_euclid({v1}, {v2}): C .so returned {c}, Rust .so returned {r}");
            }
            if v2 == i32::MAX {
                break;
            }
            v2 += 1;
        }
    }
}

/// C35 (stress): exhaustive sweep of EVERY one of the 2^32 possible dividends
/// against the hardest divisors (the mirror of C34).
#[test]
fn config_c35_exhaustive_dividend_sweep_for_critical_divisors() {
    let p = libs();
    for v2 in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX] {
        let mut v1: i32 = i32::MIN;
        loop {
            let (c, r) = unsafe { ((p.c)(v1, v2), (p.rust)(v1, v2)) };
            if c != r {
                panic!("div_euclid({v1}, {v2}): C .so returned {c}, Rust .so returned {r}");
            }
            if v1 == i32::MAX {
                break;
            }
            v1 += 1;
        }
    }
}

// ===========================================================================
// Phase C — ERRORS.md rows
// ===========================================================================

/// E1: `v2 == 0` — the library's only rejection. Must return the exact
/// sentinel `0` for every dividend, and must never trap.
#[test]
fn error_e1_divisor_zero() {
    for &v1 in BOUNDARY {
        check_exact(v1, 0, 0);
    }
    let mut rng = Rng::new(SEED ^ 0xE1);
    for _ in 0..RANDOM_CASES {
        check_exact(rng.next_i32(), 0, 0);
        check_exact(rng.biased_i32(), 0, 0);
    }
}

/// E2: `v1 >= 0 && v2 == INT_MIN` — the `v2 != INT_MIN` guard fails; expected
/// `q = 0, r = v1 >= 0` → return `0`.
#[test]
fn error_e2_nonneg_dividend_intmin_divisor() {
    check_exact(0, i32::MIN, 0);
    check_exact(1, i32::MIN, 0);
    check_exact(i32::MAX, i32::MIN, 0);
    check_exact(i32::MAX - 1, i32::MIN, 0);
    let mut rng = Rng::new(SEED ^ 0xE2);
    for _ in 0..RANDOM_CASES {
        check_exact(rng.range_i32(0, i32::MAX), i32::MIN, 0);
    }
}

/// E3: `v1 == INT_MIN && v2 == INT_MIN` — both guards fail; `q = 1, r = 0`.
#[test]
fn error_e3_intmin_over_intmin() {
    check_exact(i32::MIN, i32::MIN, 1);
}

/// E4: `v1 == INT_MIN && v2 > 0` — the `-(v1 + v2)` workaround path.
#[test]
fn error_e4_intmin_dividend_positive_divisor() {
    check_all(&[
        (i32::MIN, 1),
        (i32::MIN, 2),
        (i32::MIN, 3),
        (i32::MIN, i32::MAX),
        (i32::MIN, i32::MAX - 1),
    ]);
    let mut rng = Rng::new(SEED ^ 0xE4);
    for _ in 0..RANDOM_CASES {
        check(i32::MIN, rng.range_i32(1, i32::MAX));
    }
}

/// E5: `v1 == INT_MIN && INT_MIN < v2 < 0` — the `-(v1 - v2)` workaround path.
#[test]
fn error_e5_intmin_dividend_negative_divisor() {
    check_all(&[
        (i32::MIN, -1),
        (i32::MIN, -2),
        (i32::MIN, -3),
        (i32::MIN, i32::MIN + 1),
        (i32::MIN, i32::MIN + 2),
    ]);
    let mut rng = Rng::new(SEED ^ 0xE5);
    for _ in 0..RANDOM_CASES {
        check(i32::MIN, rng.range_i32(i32::MIN + 1, -1));
    }
}

/// E6: `INT_MIN < v1 < 0 && v2 == INT_MIN` — `q = 1, r = v1 - q*v2 > 0`
/// (comma-operator sequencing), so the result is exactly `1`.
#[test]
fn error_e6_negative_dividend_intmin_divisor() {
    check_exact(-1, i32::MIN, 1);
    check_exact(i32::MIN + 1, i32::MIN, 1);
    let mut rng = Rng::new(SEED ^ 0xE6);
    for _ in 0..RANDOM_CASES {
        check_exact(rng.range_i32(i32::MIN + 1, -1), i32::MIN, 1);
    }
}

/// E7: `v1 == INT_MIN && v2 == 0` — the zero guard fires before any INT_MIN
/// handling.
#[test]
fn error_e7_intmin_over_zero() {
    check_exact(i32::MIN, 0, 0);
}

/// E8: `v1 == INT_MIN && v2 == -1` — the classic overflow trap; the C reaches
/// it only via the E5 path and never executes `INT_MIN / -1`.
#[test]
fn error_e8_intmin_over_minus_one() {
    check(i32::MIN, -1);
}

/// E9: `v1 == 0` with any non-zero divisor.
#[test]
fn error_e9_zero_dividend() {
    for &v2 in BOUNDARY {
        if v2 != 0 {
            check_exact(0, v2, 0);
        }
    }
    let mut rng = Rng::new(SEED ^ 0xE9);
    for _ in 0..RANDOM_CASES {
        check_exact(0, rng.nonzero_i32(), 0);
    }
}

/// Generic FFI boundary sweep: since the signature is scalar-only there are no
/// pointers, lengths or enums, so "one step past the valid range" means the
/// `i32` extremes and their neighbours — exhaustively crossed here, plus a
/// sweep over every divisor for the three extreme dividends.
#[test]
fn errors_full_boundary_cross_product() {
    let extremes = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 2,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX - 2,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &v1 in &extremes {
        for &v2 in &extremes {
            check(v1, v2);
        }
    }
    // Dense sweep of every divisor magnitude bit pattern against INT_MIN.
    for k in 0..32u32 {
        let m = if k == 31 { i32::MIN } else { 1i32 << k };
        for v1 in [i32::MIN, i32::MIN + 1, i32::MAX, 0, -1, 1] {
            check(v1, m);
            check(v1, m.wrapping_neg());
            check(v1, m.wrapping_add(1));
            check(v1, m.wrapping_sub(1));
        }
    }
}

/// Sanity: the loaded Rust `.so` really is the exported symbol (not a
/// statically linked copy) and both handles resolve to distinct addresses.
#[test]
fn meta_both_symbols_loaded_from_distinct_shared_objects() {
    let p = libs();
    let c_addr = p.c as usize;
    let rust_addr = p.rust as usize;
    assert_ne!(
        c_addr, rust_addr,
        "C and Rust div_euclid resolved to the same address — only one .so was loaded"
    );
}
