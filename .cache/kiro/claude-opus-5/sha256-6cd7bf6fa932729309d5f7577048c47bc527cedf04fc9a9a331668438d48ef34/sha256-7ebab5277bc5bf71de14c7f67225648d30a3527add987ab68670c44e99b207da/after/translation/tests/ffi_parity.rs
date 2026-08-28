//! Differential test: loads the C `.so` and the Rust `cdylib` through
//! `libloading` and compares `ldexp_q2` bit-for-bit across the FFI boundary.
//!
//! The Rust side is *never* called directly — only through its exported
//! `#[no_mangle]` symbol, exactly like an external C caller would.

use std::ffi::c_int;
use std::path::PathBuf;

use libloading::{Library, Symbol};

type LdexpQ2 = unsafe extern "C" fn(f32, c_int) -> f32;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_library_path() -> PathBuf {
    let build_dir = workspace_root().join("c_src/build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(path);
            }
        }
    }
    candidates.sort();
    candidates.pop().unwrap_or_else(|| {
        panic!(
            "no C shared library found in {}; build it with \
             `cd c_src && mkdir -p build && cd build && cmake .. \
             -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`",
            build_dir.display()
        )
    })
}

fn rust_library_path() -> PathBuf {
    // The integration-test binary lives in target/<profile>/deps/, so the
    // cdylib is two levels up from the executable.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>")
        .to_path_buf();

    for name in ["libldexp_q2_lib.so", "libtranslation.so"] {
        let candidate = profile_dir.join(name);
        if candidate.exists() {
            return candidate;
        }
    }
    panic!(
        "no Rust cdylib found in {} (looked for libldexp_q2_lib.so)",
        profile_dir.display()
    );
}

struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c: LdexpQ2,
    rust: LdexpQ2,
}

impl Pair {
    fn load() -> Self {
        unsafe {
            let c_path = c_library_path();
            let rust_path = rust_library_path();

            let c_lib = Library::new(&c_path)
                .unwrap_or_else(|e| panic!("failed to load {}: {e}", c_path.display()));
            let rust_lib = Library::new(&rust_path)
                .unwrap_or_else(|e| panic!("failed to load {}: {e}", rust_path.display()));

            let c_sym: Symbol<LdexpQ2> = c_lib
                .get(b"ldexp_q2\0")
                .expect("C .so must export `ldexp_q2`");
            let rust_sym: Symbol<LdexpQ2> = rust_lib
                .get(b"ldexp_q2\0")
                .expect("Rust .so must export `ldexp_q2`");

            let c = *c_sym;
            let rust = *rust_sym;

            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }

    /// Returns the raw bit patterns produced by each implementation.
    fn call(&self, y: f32, exp_q2: c_int) -> (u32, u32) {
        let c = unsafe { (self.c)(y, exp_q2) };
        let r = unsafe { (self.rust)(y, exp_q2) };
        (c.to_bits(), r.to_bits())
    }

    fn assert_same(&self, y: f32, exp_q2: c_int) {
        let (c, r) = self.call(y, exp_q2);
        assert_eq!(
            c,
            r,
            "mismatch for ldexp_q2(y = {y:e} [bits 0x{ybits:08x}], exp_q2 = {exp_q2}): \
             C = {cf:e} (0x{c:08x}), Rust = {rf:e} (0x{r:08x})",
            ybits = y.to_bits(),
            cf = f32::from_bits(c),
            rf = f32::from_bits(r),
        );
    }
}

/// A spread of interesting `y` values: zeros, subnormals, normals at both
/// extremes, powers of two, odd mantissas, infinities and NaNs.
fn interesting_y() -> Vec<f32> {
    let mut v: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.0,
        -2.0,
        3.0,
        -3.0,
        1.5,
        -1.5,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
        f32::EPSILON,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        1.0e-30,
        1.0e30,
        -1.0e30,
        123.456,
        -987.654_3,
        core::f32::consts::PI,
        -core::f32::consts::E,
    ];

    // Subnormals and boundary bit patterns.
    for bits in [
        0x0000_0001u32, // smallest positive subnormal
        0x0000_0002,
        0x007f_ffff, // largest subnormal
        0x0080_0000, // smallest normal
        0x7f7f_ffff, // FLT_MAX
        0x7f80_0000, // +inf
        0x7f80_0001, // signalling NaN
        0x7fc0_0000, // quiet NaN
        0x7fff_ffff,
        0x8000_0001,
        0xff7f_ffff,
        0xffc0_0000,
    ] {
        v.push(f32::from_bits(bits));
        v.push(f32::from_bits(bits ^ 0x8000_0000));
    }

    // Deterministic pseudo-random bit patterns (xorshift).
    let mut state: u32 = 0x1234_5678;
    for _ in 0..64 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        v.push(f32::from_bits(state));
    }

    v
}

#[test]
fn exports_expected_symbol() {
    // Loading already asserts the symbol exists in both libraries.
    let pair = Pair::load();
    pair.assert_same(1.0, 4);
}

/// The dense region: every quarter-exponent step through the whole
/// single-iteration range and a few multi-iteration wraps.
#[test]
fn dense_exponent_sweep() {
    let pair = Pair::load();
    let ys = interesting_y();
    for exp_q2 in -600..=600 {
        for &y in &ys {
            pair.assert_same(y, exp_q2);
        }
    }
}

/// Exactly the boundaries of the `min(exp_q2, 120)` clamp and the `e & 3` /
/// `e >> 2` splits, where an off-by-one in the translation would show up.
#[test]
fn clamp_and_shift_boundaries() {
    let pair = Pair::load();
    let ys = interesting_y();

    let mut exps: Vec<c_int> = Vec::new();
    for k in -8..=40 {
        let base = 120 * k;
        for delta in -5..=5 {
            exps.push(base + delta);
        }
    }
    // Every possible `e & 3` / `e >> 2` combination for a single iteration.
    for e in -128..=121 {
        exps.push(e);
    }
    exps.sort_unstable();
    exps.dedup();

    for &exp_q2 in &exps {
        for &y in &ys {
            pair.assert_same(y, exp_q2);
        }
    }
}

/// Negative and extreme `exp_q2`, including `INT_MIN`, where the C code shifts
/// by a negative amount and the loop terminates after a single iteration.
#[test]
fn negative_and_extreme_exponents() {
    let pair = Pair::load();
    let ys = interesting_y();

    let mut exps: Vec<c_int> = vec![
        c_int::MIN,
        c_int::MIN + 1,
        c_int::MIN + 2,
        c_int::MIN + 3,
        c_int::MIN + 4,
        -2_000_000_000,
        -1_073_741_824, // -(1 << 30)
        -1_073_741_823,
        -536_870_912,
        -100_000_000,
        -1_000_000,
        -65_536,
        -4_096,
        -256,
        -128,
        -127,
        -4,
        -3,
        -2,
        -1,
        0,
    ];
    // Multiples of 4 and 32 in the negative range exercise the shift-count
    // masking (`& 31`) that the hardware performs.
    for k in 1..=64 {
        exps.push(-4 * k);
        exps.push(-32 * k);
        exps.push(-128 * k);
    }
    exps.sort_unstable();
    exps.dedup();

    for &exp_q2 in &exps {
        for &y in &ys {
            pair.assert_same(y, exp_q2);
        }
    }
}

/// Large positive `exp_q2` values force many loop iterations; keep the counts
/// bounded so the test stays fast.
#[test]
fn large_positive_exponents() {
    let pair = Pair::load();
    let ys = interesting_y();

    let exps: Vec<c_int> = vec![
        601, 1_000, 1_200, 1_201, 4_096, 10_000, 12_000, 12_001, 65_536, 100_000, 120_000, 500_000,
        1_000_000,
    ];

    for &exp_q2 in &exps {
        for &y in &ys {
            pair.assert_same(y, exp_q2);
        }
    }

    // A handful of very large values (millions of iterations each), only for a
    // few `y` values so the runtime stays reasonable.
    for &exp_q2 in &[10_000_000, 100_000_000, c_int::MAX - 1, c_int::MAX] {
        for &y in &[1.0f32, -1.0, 0.0, f32::MIN_POSITIVE, f32::NAN] {
            pair.assert_same(y, exp_q2);
        }
    }
}

/// Randomised fuzzing over both arguments.
#[test]
fn randomised_fuzz() {
    let pair = Pair::load();

    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for _ in 0..200_000 {
        let r = next();
        let y = f32::from_bits((r >> 32) as u32);
        // Bias towards small exponents (the common case) but occasionally use
        // the full 32-bit range.
        let raw = r as u32 as i32;
        let exp_q2 = match r % 4 {
            0 => raw % 2_048,
            1 => raw % 300,
            2 => raw % 32,
            _ => raw / 1_024, // still large, but bounded loop counts
        };
        pair.assert_same(y, exp_q2);
    }
}

/// Heavy sweep: walks the whole 32-bit `f32` space with a stride, crossed with
/// every `e & 3` / `e >> 2` combination plus negative exponents. Ignored by
/// default; run with `cargo test --release -- --ignored`.
#[test]
#[ignore = "slow: ~4M f32 bit patterns x many exponents"]
fn exhaustive_stride_sweep() {
    let pair = Pair::load();

    let exps: Vec<c_int> = vec![
        -1_073_741_824,
        -1_000,
        -121,
        -120,
        -33,
        -32,
        -5,
        -4,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        119,
        120,
        121,
        124,
        240,
        241,
        c_int::MIN,
    ];

    // 2^32 / 4096 == 1_048_576 distinct y values.
    const STRIDE: u32 = 4096;

    for &exp_q2 in &exps {
        let mut bits: u32 = 0;
        loop {
            pair.assert_same(f32::from_bits(bits), exp_q2);
            match bits.checked_add(STRIDE) {
                Some(next) => bits = next,
                None => break,
            }
        }
        // Also hit the very top of the range, which the stride misses.
        for bits in (u32::MAX - 8)..=u32::MAX {
            pair.assert_same(f32::from_bits(bits), exp_q2);
        }
    }
}
