//! Differential tests: load BOTH the C `.so` and the Rust `.so` through
//! `libloading` and compare `rgb_to_hsv` outputs bit-for-bit.
//!
//! Neither implementation is ever called directly as a Rust function — both go
//! through the dynamic-library FFI boundary, so the `#[no_mangle]` export
//! wrapper is under test too.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

type RgbToHsv = unsafe extern "C" fn(*mut f32, *const f32);

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn find_c_so() -> PathBuf {
    let build = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("so") {
                candidates.push(p);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no .so found in {}; build the C library first:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO_PATH") {
        return PathBuf::from(p);
    }
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    // The test binary lives in target/<profile>/deps/, so prefer the profile
    // this test was built under, then fall back to the other one.
    let exe = std::env::current_exe().expect("current_exe");
    let mut order: Vec<PathBuf> = Vec::new();
    if let Some(profile_dir) = exe.parent().and_then(|deps| deps.parent()) {
        order.push(profile_dir.join("librgb_to_hsv_lib.so"));
    }
    order.push(target.join("debug").join("librgb_to_hsv_lib.so"));
    order.push(target.join("release").join("librgb_to_hsv_lib.so"));
    for p in &order {
        if p.is_file() {
            return p.clone();
        }
    }
    panic!(
        "Rust cdylib not found; looked in {:?}. Build it with `cargo build` \
         (or `cargo build --release`).",
        order
    );
}

/// Guard against the cargo pitfall that `cargo test` does NOT rebuild a
/// `crate-type = ["cdylib"]` library target: without this check the tests would
/// happily dlopen a stale `.so` from a previous build and pass vacuously.
///
/// We require the `.so` to be at least as new as every Rust source file. Build
/// with `./run_tests.sh`, or `cargo build --release` before `cargo test`.
fn assert_so_is_fresh(so: &Path) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", so.display()));
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![src_dir];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                    if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                        if newest.as_ref().map_or(true, |(_, n)| t > *n) {
                            newest = Some((p, t));
                        }
                    }
                }
            }
        }
    }
    if let Some((path, t)) = newest {
        assert!(
            t <= so_mtime,
            "STALE Rust .so: {} was modified after {} was built.\n\
             `cargo test` does not rebuild a cdylib-only lib target, so the \n\
             differential tests would compare against an OLD binary and pass \n\
             vacuously. Run `cargo build --release` (or ./run_tests.sh) first.",
            path.display(),
            so.display()
        );
    }
}

/// Holds both libraries plus the resolved `rgb_to_hsv` symbol from each.
struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c_fn: RgbToHsv,
    rust_fn: RgbToHsv,
}

impl Pair {
    fn load() -> Pair {
        unsafe {
            let c_path = find_c_so();
            let r_path = find_rust_so();
            assert_so_is_fresh(&r_path);
            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display()));
            let rust_lib = Library::new(&r_path)
                .unwrap_or_else(|e| panic!("dlopen {}: {e}", r_path.display()));
            let c_sym: Symbol<RgbToHsv> = c_lib
                .get(b"rgb_to_hsv\0")
                .expect("C .so must export rgb_to_hsv");
            let r_sym: Symbol<RgbToHsv> = rust_lib
                .get(b"rgb_to_hsv\0")
                .expect("Rust .so must export rgb_to_hsv");
            let c_fn = *c_sym;
            let rust_fn = *r_sym;
            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_fn,
                rust_fn,
            }
        }
    }

    /// Call C through FFI with a fresh, poisoned destination buffer.
    fn call_c(&self, src: [f32; 3]) -> [f32; 3] {
        let mut dest = [f32::from_bits(0xDEAD_BEEF); 3];
        unsafe { (self.c_fn)(dest.as_mut_ptr(), src.as_ptr()) };
        dest
    }

    /// Call Rust through FFI with a fresh, poisoned destination buffer.
    fn call_rust(&self, src: [f32; 3]) -> [f32; 3] {
        let mut dest = [f32::from_bits(0xDEAD_BEEF); 3];
        unsafe { (self.rust_fn)(dest.as_mut_ptr(), src.as_ptr()) };
        dest
    }
}

// ---------------------------------------------------------------------------
// Bit-exact comparison
// ---------------------------------------------------------------------------

fn bits(v: [f32; 3]) -> [u32; 3] {
    [v[0].to_bits(), v[1].to_bits(), v[2].to_bits()]
}

fn show(v: [f32; 3]) -> String {
    format!(
        "[{:e} (0x{:08x}), {:e} (0x{:08x}), {:e} (0x{:08x})]",
        v[0],
        v[0].to_bits(),
        v[1],
        v[1].to_bits(),
        v[2],
        v[2].to_bits()
    )
}

/// Assert C and Rust agree bit-for-bit (so `-0.0` vs `+0.0` and NaN payloads
/// are both distinguished) for one input.
#[track_caller]
fn assert_same(p: &Pair, row: &str, src: [f32; 3]) {
    let c = p.call_c(src);
    let r = p.call_rust(src);
    assert_eq!(
        bits(c),
        bits(r),
        "\n[{row}] divergence for src = {}\n  C    = {}\n  Rust = {}\n",
        show(src),
        show(c),
        show(r)
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in [0, 1).
    fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
    /// Uniform in [lo, hi).
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Iterations per randomized row.
const N: usize = 20_000;

// ===========================================================================
// CONFIGS.md rows
// ===========================================================================

// --- Row 1: documented happy path -----------------------------------------
#[test]
fn row01_random_unit_cube() {
    let p = Pair::load();
    let mut rng = Rng::new(0x1111_1111);
    for _ in 0..N {
        assert_same(
            &p,
            "row01",
            [rng.unit(), rng.unit(), rng.unit()],
        );
    }
}

// --- Rows 2-4: each hue branch, unique max --------------------------------

/// Build a triple where component `max_idx` is the strict maximum.
fn strict_max_at(rng: &mut Rng, max_idx: usize) -> [f32; 3] {
    loop {
        let mut v = [rng.unit(), rng.unit(), rng.unit()];
        // Force a strict maximum at max_idx.
        v[max_idx] = 1.0 + rng.unit();
        let m = v[max_idx];
        if (0..3).filter(|&i| i != max_idx).all(|i| v[i] < m) {
            return v;
        }
    }
}

#[test]
fn row02_hue_branch_r() {
    let p = Pair::load();
    let mut rng = Rng::new(0x2222_2222);
    for _ in 0..N {
        assert_same(&p, "row02", strict_max_at(&mut rng, 0));
    }
}

#[test]
fn row03_hue_branch_g() {
    let p = Pair::load();
    let mut rng = Rng::new(0x3333_3333);
    for _ in 0..N {
        assert_same(&p, "row03", strict_max_at(&mut rng, 1));
    }
}

#[test]
fn row04_hue_branch_b_else() {
    let p = Pair::load();
    let mut rng = Rng::new(0x4444_4444);
    for _ in 0..N {
        assert_same(&p, "row04", strict_max_at(&mut rng, 2));
    }
}

// --- Rows 5-7: ties, first-match branch must win --------------------------

#[test]
fn row05_tie_r_eq_g_gt_b() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5555_5555);
    for _ in 0..N {
        let hi = rng.range(0.25, 1.0);
        let lo = rng.range(-1.0, 0.2);
        assert_same(&p, "row05", [hi, hi, lo]);
    }
}

#[test]
fn row06_tie_g_eq_b_gt_r() {
    let p = Pair::load();
    let mut rng = Rng::new(0x6666_6666);
    for _ in 0..N {
        let hi = rng.range(0.25, 1.0);
        let lo = rng.range(-1.0, 0.2);
        assert_same(&p, "row06", [lo, hi, hi]);
    }
}

#[test]
fn row07_tie_r_eq_b_gt_g() {
    let p = Pair::load();
    let mut rng = Rng::new(0x7777_7777);
    for _ in 0..N {
        let hi = rng.range(0.25, 1.0);
        let lo = rng.range(-1.0, 0.2);
        assert_same(&p, "row07", [hi, lo, hi]);
    }
}

// --- Rows 8-9: hue wrap-around correction ---------------------------------

#[test]
fn row08_hue_wrap_taken() {
    let p = Pair::load();
    let mut rng = Rng::new(0x8888_8888);
    for _ in 0..N {
        // r strictly max, g < b  =>  h = (g-b)/delta < 0  =>  h += 360
        let r = rng.range(0.5, 1.0);
        let g = rng.range(0.0, 0.2);
        let b = rng.range(0.21, 0.49);
        let out = p.call_c([r, g, b]);
        assert!(
            out[0] >= 0.0 && out[0] < 360.0,
            "row08 precondition: expected wrapped hue, got {}",
            out[0]
        );
        assert_same(&p, "row08", [r, g, b]);
    }
}

#[test]
fn row09_hue_wrap_not_taken() {
    let p = Pair::load();
    let mut rng = Rng::new(0x9999_9999);
    for _ in 0..N {
        let r = rng.range(0.5, 1.0);
        let b = rng.range(0.0, 0.2);
        let g = rng.range(0.21, 0.49);
        assert_same(&p, "row09", [r, g, b]);
    }
}

// --- Rows 10-13: the `delta == 0 || max == 0` early-out --------------------

#[test]
fn row10_delta_zero_grey() {
    let p = Pair::load();
    let mut rng = Rng::new(0xAAAA_AAAA);
    for _ in 0..N {
        let g = rng.range(-1000.0, 1000.0);
        assert_same(&p, "row10", [g, g, g]);
    }
    // plus specific greys, including ones where max == 0 too
    for &g in &[0.0f32, -0.0, 1.0, -1.0, f32::MAX, f32::MIN, f32::MIN_POSITIVE] {
        assert_same(&p, "row10-fixed", [g, g, g]);
    }
}

#[test]
fn row11_both_disjuncts_black() {
    let p = Pair::load();
    assert_same(&p, "row11", [0.0, 0.0, 0.0]);
}

#[test]
fn row12_max_zero_only() {
    let p = Pair::load();
    let mut rng = Rng::new(0xBBBB_BBBB);
    for _ in 0..N {
        // all <= 0 with max exactly +0.0 and delta != 0
        let neg = -rng.range(0.001, 1000.0);
        let neg2 = -rng.range(0.0, 1000.0);
        for src in [
            [0.0, neg, neg2],
            [neg, 0.0, neg2],
            [neg, neg2, 0.0],
            [0.0, 0.0, neg],
            [neg, 0.0, 0.0],
            [0.0, neg, 0.0],
        ] {
            assert_same(&p, "row12", src);
        }
    }
}

#[test]
fn row13_all_negative_no_early_out() {
    let p = Pair::load();
    let mut rng = Rng::new(0xCCCC_CCCC);
    for _ in 0..N {
        let a = -rng.range(0.001, 100.0);
        let b = -rng.range(0.001, 100.0);
        let c = -rng.range(0.001, 100.0);
        assert_same(&p, "row13", [a, b, c]);
    }
}

// --- Row 14: signed zeros, exhaustive over all 8 sign patterns -------------

#[test]
fn row14_signed_zeros_exhaustive() {
    let p = Pair::load();
    let zeros = [0.0f32, -0.0f32];
    for &a in &zeros {
        for &b in &zeros {
            for &c in &zeros {
                assert_same(&p, "row14", [a, b, c]);
            }
        }
    }
    // -0.0 mixed with finite values in every position
    let mut rng = Rng::new(0xDDDD_DDDD);
    for _ in 0..N {
        let x = rng.range(-10.0, 10.0);
        let y = rng.range(-10.0, 10.0);
        for src in [
            [-0.0, x, y],
            [x, -0.0, y],
            [x, y, -0.0],
            [-0.0, -0.0, y],
            [x, -0.0, -0.0],
            [-0.0, y, -0.0],
        ] {
            assert_same(&p, "row14-mixed", src);
        }
    }
}

// --- Rows 15-17: NaN in one, two, three positions -------------------------

#[test]
fn row15_single_nan() {
    let p = Pair::load();
    let mut rng = Rng::new(0xEEEE_EEEE);
    for _ in 0..N {
        for pos in 0..3 {
            let mut src = [rng.range(-10.0, 10.0), rng.range(-10.0, 10.0), rng.range(-10.0, 10.0)];
            src[pos] = f32::NAN;
            assert_same(&p, "row15", src);
        }
    }
}

#[test]
fn row16_two_nans() {
    let p = Pair::load();
    let mut rng = Rng::new(0x1234_5678);
    for _ in 0..N {
        for finite_pos in 0..3 {
            let mut src = [f32::NAN; 3];
            src[finite_pos] = rng.range(-10.0, 10.0);
            assert_same(&p, "row16", src);
        }
    }
}

#[test]
fn row17_all_nan() {
    let p = Pair::load();
    assert_same(&p, "row17", [f32::NAN; 3]);
    // negative-signed NaN too
    assert_same(&p, "row17-neg", [-f32::NAN; 3]);
}

// --- Rows 18-20: infinities ----------------------------------------------

#[test]
fn row18_plus_infinity() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0BAD_F00D);
    for _ in 0..N {
        for pos in 0..3 {
            let mut src = [rng.range(-10.0, 10.0), rng.range(-10.0, 10.0), rng.range(-10.0, 10.0)];
            src[pos] = f32::INFINITY;
            assert_same(&p, "row18", src);
        }
    }
}

#[test]
fn row19_minus_infinity() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0F00_D0BA);
    for _ in 0..N {
        for pos in 0..3 {
            let mut src = [rng.range(-10.0, 10.0), rng.range(-10.0, 10.0), rng.range(-10.0, 10.0)];
            src[pos] = f32::NEG_INFINITY;
            assert_same(&p, "row19", src);
        }
    }
}

#[test]
fn row20_mixed_infinities() {
    let p = Pair::load();
    let inf = f32::INFINITY;
    let ninf = f32::NEG_INFINITY;
    let specials = [inf, ninf, 0.0f32, -0.0f32, 1.0f32, -1.0f32, f32::NAN];
    for &a in &specials {
        for &b in &specials {
            for &c in &specials {
                assert_same(&p, "row20", [a, b, c]);
            }
        }
    }
}

// --- Row 21: subnormals ---------------------------------------------------

#[test]
fn row21_subnormals() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5DB0_5DB0);
    for _ in 0..N {
        // random subnormal: zero exponent, nonzero mantissa, random sign
        let mk = |rng: &mut Rng| -> f32 {
            let sign = (rng.next_u32() & 1) << 31;
            let mant = (rng.next_u32() & 0x007F_FFFF).max(1);
            f32::from_bits(sign | mant)
        };
        let src = [mk(&mut rng), mk(&mut rng), mk(&mut rng)];
        assert_same(&p, "row21", src);
    }
    // extremes
    let tiny = f32::from_bits(1); // smallest positive subnormal
    let maxsub = f32::from_bits(0x007F_FFFF);
    for &a in &[tiny, -tiny, maxsub, -maxsub, f32::MIN_POSITIVE] {
        for &b in &[tiny, -tiny, maxsub, f32::MIN_POSITIVE] {
            for &c in &[tiny, maxsub, -maxsub, 0.0] {
                assert_same(&p, "row21-fixed", [a, b, c]);
            }
        }
    }
}

// --- Row 22: FLT_MAX / FLT_MIN / FLT_EPSILON, exhaustive 27 ---------------

#[test]
fn row22_float_boundaries_exhaustive() {
    let p = Pair::load();
    let vals = [f32::MAX, f32::MIN_POSITIVE, f32::EPSILON];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                assert_same(&p, "row22", [a, b, c]);
            }
        }
    }
    // and with FLT_MIN (== -FLT_MAX in C) folded in
    let vals2 = [f32::MAX, f32::MIN, f32::MIN_POSITIVE, -f32::MIN_POSITIVE, f32::EPSILON, -f32::EPSILON];
    for &a in &vals2 {
        for &b in &vals2 {
            for &c in &vals2 {
                assert_same(&p, "row22-signed", [a, b, c]);
            }
        }
    }
}

// --- Row 23: huge magnitudes, delta overflows ----------------------------

#[test]
fn row23_overflowing_delta() {
    let p = Pair::load();
    let mut rng = Rng::new(0x1E30_1E30);
    for _ in 0..N {
        let big = rng.range(1e30, 3.4e38);
        let negbig = -rng.range(1e30, 3.4e38);
        for src in [
            [big, negbig, 0.0],
            [negbig, big, 0.0],
            [0.0, negbig, big],
            [big, negbig, negbig],
            [f32::MAX, -f32::MAX, 0.0],
            [-f32::MAX, f32::MAX, 1.0],
        ] {
            assert_same(&p, "row23", src);
        }
    }
}

// --- Row 24: mixed magnitudes -------------------------------------------

#[test]
fn row24_mixed_magnitudes() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0FFF_1234);
    for _ in 0..N {
        // random exponent per component: spans 1e-38 .. 1e38
        let mk = |rng: &mut Rng| -> f32 {
            let e: i32 = rng.below(80) as i32 - 40;
            let m = rng.range(1.0, 10.0);
            let sign = if rng.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
            sign * m * (10f32).powi(e)
        };
        let src = [mk(&mut rng), mk(&mut rng), mk(&mut rng)];
        assert_same(&p, "row24", src);
    }
}

// --- Row 25: fully unconstrained random bit patterns ---------------------

#[test]
fn row25_random_bit_patterns() {
    let p = Pair::load();
    let mut rng = Rng::new(0xB17B_A775);
    for _ in 0..N {
        let src = [
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
            f32::from_bits(rng.next_u32()),
        ];
        assert_same(&p, "row25", src);
    }
}

/// Signalling-NaN payloads specifically (quiet bit clear, mantissa nonzero).
#[test]
fn row25b_signalling_nans() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5A5A_1234);
    let mk_snan = |rng: &mut Rng| -> f32 {
        let sign = (rng.next_u32() & 1) << 31;
        let mant = (rng.next_u32() & 0x003F_FFFF).max(1); // quiet bit (0x400000) clear
        f32::from_bits(sign | 0x7F80_0000 | mant)
    };
    for _ in 0..2000 {
        let src = [mk_snan(&mut rng), mk_snan(&mut rng), mk_snan(&mut rng)];
        assert_same(&p, "row25b", src);
        let mut mixed = [rng.range(-1.0, 1.0), rng.range(-1.0, 1.0), rng.range(-1.0, 1.0)];
        mixed[rng.below(3)] = mk_snan(&mut rng);
        assert_same(&p, "row25b-mixed", mixed);
    }
}

// --- Rows 26-29: pointer aliasing ---------------------------------------

/// Inputs that take the `delta == 0 || max == 0` EARLY-OUT path. The aliasing
/// rows must include these: a mistranslation that re-reads `src` after storing
/// into `dest` is only visible when the two overlap AND the early-out path runs,
/// so randomized chromatic values alone leave that path untested under aliasing.
fn early_out_triples() -> Vec<[f32; 3]> {
    let mut v = vec![
        [0.0, 0.0, 0.0],       // black: both disjuncts
        [-0.0, -0.0, -0.0],    // signed-zero black
        [0.0, -0.0, 0.0],      // mixed signed zeros
        [1.0, 1.0, 1.0],       // positive grey
        [0.5, 0.5, 0.5],
        [255.0, 255.0, 255.0],
        [-5.0, -5.0, -5.0],    // NEGATIVE grey: v is negative, so a recomputed
        [-1.0, -1.0, -1.0],    // max after clobbering dest[0..2] differs
        [f32::MIN, f32::MIN, f32::MIN],
        [0.0, -1.0, -2.0],     // max == 0 with delta != 0
        [-1.0, 0.0, -2.0],
        [-1.0, -2.0, 0.0],
        [-0.0, -1.0, -2.0],    // max == -0.0 satisfies `max == 0`
        [0.0, 0.0, -3.0],
        [-3.0, 0.0, 0.0],
    ];
    // Randomized greys and max==0 shapes, fixed seed.
    let mut rng = Rng::new(0xEA12_0007);
    for _ in 0..2000 {
        let g = rng.range(-1000.0, 1000.0);
        v.push([g, g, g]);
        let n1 = -rng.range(0.0, 1000.0);
        let n2 = -rng.range(0.0, 1000.0);
        v.push([0.0, n1, n2]);
        v.push([n1, 0.0, n2]);
        v.push([n1, n2, 0.0]);
    }
    v
}


#[test]
fn row26_full_aliasing_dest_eq_src() {
    let p = Pair::load();
    let mut rng = Rng::new(0xA11A_5111);
    // Chromatic randoms AND every early-out-taking triple.
    let mut inputs: Vec<[f32; 3]> = (0..N)
        .map(|_| [rng.range(-5.0, 5.0), rng.range(-5.0, 5.0), rng.range(-5.0, 5.0)])
        .collect();
    inputs.extend(early_out_triples());
    for src in inputs {

        let mut cbuf = src;
        unsafe { (p.c_fn)(cbuf.as_mut_ptr(), cbuf.as_ptr()) };
        let mut rbuf = src;
        unsafe { (p.rust_fn)(rbuf.as_mut_ptr(), rbuf.as_ptr()) };

        assert_eq!(
            bits(cbuf),
            bits(rbuf),
            "[row26] aliased dest==src diverged for {}\n  C    = {}\n  Rust = {}",
            show(src),
            show(cbuf),
            show(rbuf)
        );
    }
}

#[test]
fn row27_partial_overlap_dest_after_src() {
    let p = Pair::load();
    let mut rng = Rng::new(0x07E5_0001);
    let mut bases: Vec<[f32; 4]> = (0..N)
        .map(|_| {
            [
                rng.range(-5.0, 5.0),
                rng.range(-5.0, 5.0),
                rng.range(-5.0, 5.0),
                rng.range(-5.0, 5.0),
            ]
        })
        .collect();
    // Early-out triples in the src window, with both tail choices.
    for t in early_out_triples() {
        bases.push([t[0], t[1], t[2], t[0]]);
        bases.push([t[0], t[1], t[2], -7.5]);
    }
    for base in bases {
        // src = &buf[0], dest = &buf[1]
        let mut cbuf = base;
        unsafe {
            let sp = cbuf.as_ptr();
            let dp = cbuf.as_mut_ptr().add(1);
            (p.c_fn)(dp, sp);
        }
        let mut rbuf = base;
        unsafe {
            let sp = rbuf.as_ptr();
            let dp = rbuf.as_mut_ptr().add(1);
            (p.rust_fn)(dp, sp);
        }
        assert_eq!(
            cbuf.map(f32::to_bits),
            rbuf.map(f32::to_bits),
            "[row27] dest==src+1 diverged for base {:?}\n  C    = {:?}\n  Rust = {:?}",
            base,
            cbuf,
            rbuf
        );
    }
}

#[test]
fn row28_partial_overlap_dest_before_src() {
    let p = Pair::load();
    let mut rng = Rng::new(0x07E5_0002);
    let mut bases: Vec<[f32; 4]> = (0..N)
        .map(|_| {
            [
                rng.range(-5.0, 5.0),
                rng.range(-5.0, 5.0),
                rng.range(-5.0, 5.0),
                rng.range(-5.0, 5.0),
            ]
        })
        .collect();
    for t in early_out_triples() {
        bases.push([-7.5, t[0], t[1], t[2]]);
        bases.push([t[2], t[0], t[1], t[2]]);
    }
    for base in bases {
        // src = &buf[1], dest = &buf[0]
        let mut cbuf = base;
        unsafe {
            let sp = cbuf.as_ptr().add(1);
            let dp = cbuf.as_mut_ptr();
            (p.c_fn)(dp, sp);
        }
        let mut rbuf = base;
        unsafe {
            let sp = rbuf.as_ptr().add(1);
            let dp = rbuf.as_mut_ptr();
            (p.rust_fn)(dp, sp);
        }
        assert_eq!(
            cbuf.map(f32::to_bits),
            rbuf.map(f32::to_bits),
            "[row28] dest==src-1 diverged for base {:?}\n  C    = {:?}\n  Rust = {:?}",
            base,
            cbuf,
            rbuf
        );
    }
}

#[test]
fn row29_src_in_middle_of_larger_buffer() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0FF5_E700);
    for _ in 0..N {
        let mut big = [0f32; 16];
        for slot in big.iter_mut() {
            *slot = rng.range(-100.0, 100.0);
        }
        // Half the iterations plant an early-out triple at the read offset.
        let off = rng.below(13); // 0..=12, leaves 3 readable
        if rng.below(2) == 0 {
            let eo = early_out_triples();
            let t = eo[rng.below(eo.len())];
            big[off] = t[0];
            big[off + 1] = t[1];
            big[off + 2] = t[2];
        }
        let mut cdest = [f32::from_bits(0xDEAD_BEEF); 8];
        let mut rdest = [f32::from_bits(0xDEAD_BEEF); 8];
        let doff = rng.below(6); // 0..=5, leaves 3 writable
        unsafe {
            (p.c_fn)(cdest.as_mut_ptr().add(doff), big.as_ptr().add(off));
            (p.rust_fn)(rdest.as_mut_ptr().add(doff), big.as_ptr().add(off));
        }
        assert_eq!(
            cdest.map(f32::to_bits),
            rdest.map(f32::to_bits),
            "[row29] offset src/dest diverged (off={off}, doff={doff}) for {:?}",
            &big[off..off + 3]
        );
    }
}

// --- Row 30: exhaustive branch cross-product grid ------------------------

#[test]
fn row30_quantized_grid_exhaustive() {
    let p = Pair::load();
    let vals = [0.0f32, 0.5, 1.0, -1.0, 2.0];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                assert_same(&p, "row30", [a, b, c]);
            }
        }
    }
    // A wider deterministic grid: every combination of a 9-value ladder (729).
    let ladder = [-2.0f32, -1.0, -0.5, -0.0, 0.0, 0.5, 1.0, 2.0, 255.0];
    for &a in &ladder {
        for &b in &ladder {
            for &c in &ladder {
                assert_same(&p, "row30-ladder", [a, b, c]);
            }
        }
    }
    // Byte-scale RGB, exhaustive over a coarse 0..=255 step-17 grid.
    let bytes: Vec<f32> = (0..=255u32).step_by(17).map(|v| v as f32 / 255.0).collect();
    for &a in &bytes {
        for &b in &bytes {
            for &c in &bytes {
                assert_same(&p, "row30-bytes", [a, b, c]);
            }
        }
    }
}
