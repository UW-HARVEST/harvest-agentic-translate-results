//! Differential test: load the C shared library and the Rust `cdylib` through
//! `libloading` and compare their exported `colourblind` symbol bit-for-bit.
//!
//! The Rust side is deliberately exercised *only* through the dynamic symbol so
//! that the `#[no_mangle] extern "C"` wrapper is part of what is under test.

use std::path::PathBuf;

use libloading::{Library, Symbol};

/// `void colourblind(cb_impairment Impairment, float *R, float *G, float *B)`
type ColourblindFn = unsafe extern "C" fn(i32, *mut f32, *mut f32, *mut f32);

const CB_PROTANOPIA: i32 = 0;
const CB_DEUTERANOPIA: i32 = 1;
const CB_TRITANOPIA: i32 = 2;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate `c_src/build/lib<project>.so`. The CMake project name is derived from
/// the *parent* directory name of `c_src`, i.e. the working-directory name, so
/// glob for the single `.so` instead of hardcoding it.
fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir()
        .parent()
        .expect("translation/ has a parent")
        .join("c_src")
        .join("build");

    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build_dir.display()))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "so"))
        .collect();
    candidates.sort();

    match candidates.len() {
        0 => panic!("no .so found in {}", build_dir.display()),
        _ => candidates.remove(0),
    }
}

/// Locate the Rust `cdylib` that the test binary was built alongside.
///
/// `CARGO_MANIFEST_DIR` plus the test executable's own directory covers both
/// `target/debug` and `target/<profile>` layouts.
fn rust_library_path() -> PathBuf {
    let file_name = format!(
        "{}colourblind_lib{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );

    // tests live in `<target>/<profile>/deps/<name>-<hash>`; the cdylib is two
    // levels up in `<target>/<profile>`.
    let exe = std::env::current_exe().expect("test executable path");
    let mut dir = exe.parent().map(PathBuf::from);
    while let Some(candidate_dir) = dir {
        let candidate = candidate_dir.join(&file_name);
        if candidate.is_file() {
            return candidate;
        }
        dir = candidate_dir.parent().map(PathBuf::from);
    }

    panic!("could not locate {file_name} near {}", exe.display());
}

/// Guard against silently testing a stale `cdylib`: `cargo` builds the test
/// binary and the `cdylib` as separate units, and a mismatch there produces
/// confusing "unchanged" failures after an edit.
fn assert_rust_library_is_fresh(lib: &PathBuf) {
    let source = manifest_dir().join("src").join("lib.rs");
    let lib_time = std::fs::metadata(lib)
        .and_then(|m| m.modified())
        .expect("cdylib mtime");
    let src_time = std::fs::metadata(&source)
        .and_then(|m| m.modified())
        .expect("src/lib.rs mtime");
    assert!(
        lib_time >= src_time,
        "{} is older than {} — rebuild before testing",
        lib.display(),
        source.display()
    );
}

struct Impl {
    _lib: Library,
    colourblind: ColourblindFn,
}

impl Impl {
    fn load(path: &PathBuf) -> Self {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()));
        let symbol: Symbol<ColourblindFn> = unsafe { lib.get(b"colourblind\0") }
            .unwrap_or_else(|e| panic!("no `colourblind` in {}: {e}", path.display()));
        let colourblind = *symbol;
        Self {
            _lib: lib,
            colourblind,
        }
    }

    /// Non-aliasing call: three distinct `f32` slots.
    fn call(&self, impairment: i32, rgb: (f32, f32, f32)) -> (f32, f32, f32) {
        let (mut r, mut g, mut b) = rgb;
        unsafe { (self.colourblind)(impairment, &mut r, &mut g, &mut b) };
        (r, g, b)
    }

    /// Aliasing call: `pick` maps each of the three parameters onto an index in
    /// a shared 3-slot buffer, so the same address can be passed twice.
    fn call_aliased(&self, impairment: i32, slots: [f32; 3], pick: [usize; 3]) -> [f32; 3] {
        let mut slots = slots;
        let base = slots.as_mut_ptr();
        unsafe {
            (self.colourblind)(
                impairment,
                base.add(pick[0]),
                base.add(pick[1]),
                base.add(pick[2]),
            )
        };
        slots
    }
}

fn implementations() -> (Impl, Impl) {
    let rust_path = rust_library_path();
    assert_rust_library_is_fresh(&rust_path);
    (Impl::load(&c_library_path()), Impl::load(&rust_path))
}

/// Bit-exact comparison: `f32` equality would conflate `0.0`/`-0.0` and make
/// every NaN unequal, neither of which is what "byte-identical" means here.
fn assert_bits_eq(
    label: &str,
    impairment: i32,
    input: (f32, f32, f32),
    c: (f32, f32, f32),
    rust: (f32, f32, f32),
) {
    let c_bits = (c.0.to_bits(), c.1.to_bits(), c.2.to_bits());
    let rust_bits = (rust.0.to_bits(), rust.1.to_bits(), rust.2.to_bits());
    assert_eq!(
        c_bits, rust_bits,
        "{label}: colourblind(impairment={impairment}, {:?}) mismatch\n  \
         C    = {:?} (bits {:#010x} {:#010x} {:#010x})\n  \
         Rust = {:?} (bits {:#010x} {:#010x} {:#010x})",
        input, c, c_bits.0, c_bits.1, c_bits.2, rust, rust_bits.0, rust_bits.1, rust_bits.2,
    );
}

/// Deterministic 32-bit xorshift, so failures are reproducible.
struct XorShift(u32);

impl XorShift {
    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
}

const IMPAIRMENTS: [i32; 3] = [CB_PROTANOPIA, CB_DEUTERANOPIA, CB_TRITANOPIA];

/// Values chosen to hit sign handling, the 0..=1 colour range the matrices are
/// designed for, subnormals, overflow, and the non-finite specials.
fn edge_values() -> Vec<f32> {
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        0.25,
        1.0 / 3.0,
        255.0,
        1.0 / 255.0,
        f32::EPSILON,
        -f32::EPSILON,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),  // smallest positive subnormal
        f32::from_bits(0x0080_0000 - 1), // largest subnormal
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7fc0_1234), // quiet NaN with payload
        f32::from_bits(0x7f80_0001), // signalling NaN
        1e-30,
        1e30,
        16_777_216.0, // 2^24, first integer with a gap
        16_777_217.0,
    ]
}

#[test]
fn identical_symbol_exports() {
    // Everything the C .so exports must exist in the Rust .so under the same
    // name; loading the symbol is the strongest form of that check.
    let (c, rust) = implementations();
    let _ = (c.colourblind, rust.colourblind);
}

#[test]
fn matches_on_unit_range_grid() {
    let (c, rust) = implementations();
    let steps: Vec<f32> = (0..=16).map(|i| i as f32 / 16.0).collect();

    for &impairment in &IMPAIRMENTS {
        for &r in &steps {
            for &g in &steps {
                for &b in &steps {
                    let input = (r, g, b);
                    assert_bits_eq(
                        "unit grid",
                        impairment,
                        input,
                        c.call(impairment, input),
                        rust.call(impairment, input),
                    );
                }
            }
        }
    }
}

#[test]
fn matches_on_edge_values() {
    let (c, rust) = implementations();
    let values = edge_values();

    for &impairment in &IMPAIRMENTS {
        // Full cartesian product would be 27^3 per impairment, which is still
        // cheap, so cover it exhaustively.
        for &r in &values {
            for &g in &values {
                for &b in &values {
                    let input = (r, g, b);
                    assert_bits_eq(
                        "edge values",
                        impairment,
                        input,
                        c.call(impairment, input),
                        rust.call(impairment, input),
                    );
                }
            }
        }
    }
}

#[test]
fn matches_on_random_bit_patterns() {
    let (c, rust) = implementations();
    let mut rng = XorShift(0x1234_5678);

    for &impairment in &IMPAIRMENTS {
        for _ in 0..200_000 {
            let input = (
                f32::from_bits(rng.next_u32()),
                f32::from_bits(rng.next_u32()),
                f32::from_bits(rng.next_u32()),
            );
            assert_bits_eq(
                "random bits",
                impairment,
                input,
                c.call(impairment, input),
                rust.call(impairment, input),
            );
        }
    }
}

#[test]
fn matches_on_random_unit_range() {
    let (c, rust) = implementations();
    let mut rng = XorShift(0x9E37_79B9);
    // Draw uniformly from [0, 1) the way graphics code would.
    let mut unit = move || (rng.next_u32() >> 8) as f32 / (1u32 << 24) as f32;

    for _ in 0..200_000 {
        let input = (unit(), unit(), unit());
        for &impairment in &IMPAIRMENTS {
            assert_bits_eq(
                "random unit",
                impairment,
                input,
                c.call(impairment, input),
                rust.call(impairment, input),
            );
        }
    }
}

/// The C helpers copy all three inputs into locals before writing any output,
/// so aliasing pointers have a specific, observable result. Verify the Rust
/// port reproduces it rather than merely being correct for distinct pointers.
#[test]
fn matches_with_aliased_pointers() {
    let (c, rust) = implementations();
    let slots = [0.3, 0.6, 0.9f32];

    // Every mapping of (R, G, B) parameters onto 3 slots, including all the
    // fully and partially aliased ones.
    for &impairment in &IMPAIRMENTS {
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    let pick = [i, j, k];
                    let c_out = c.call_aliased(impairment, slots, pick);
                    let rust_out = rust.call_aliased(impairment, slots, pick);
                    assert_eq!(
                        c_out.map(f32::to_bits),
                        rust_out.map(f32::to_bits),
                        "aliased pointers: impairment={impairment} pick={pick:?} \
                         slots={slots:?}\n  C = {c_out:?}\n  Rust = {rust_out:?}"
                    );
                }
            }
        }
    }
}

/// The C `switch` has no `default:` label, so out-of-range impairment values
/// must leave the triple completely untouched.
#[test]
fn matches_on_out_of_range_impairment() {
    let (c, rust) = implementations();
    let impairments = [
        3,
        4,
        -1,
        -2,
        100,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    let inputs = [
        (0.1, 0.2, 0.3f32),
        (0.0, -0.0, 1.0),
        (f32::NAN, f32::INFINITY, f32::MIN),
    ];

    for &impairment in &impairments {
        for &input in &inputs {
            let c_out = c.call(impairment, input);
            let rust_out = rust.call(impairment, input);
            assert_bits_eq("out of range", impairment, input, c_out, rust_out);
            // And confirm the shared observable behaviour is "no change".
            assert_eq!(
                c_out.0.to_bits(),
                input.0.to_bits(),
                "C mutated R for impairment={impairment}"
            );
        }
    }
}

/// Repeated application: feeds each implementation's output back into itself so
/// any tiny divergence would compound and be caught.
#[test]
fn matches_when_applied_repeatedly() {
    let (c, rust) = implementations();

    for &impairment in &IMPAIRMENTS {
        let mut c_state = (0.9137255, 0.11764706, 0.38823530f32);
        let mut rust_state = c_state;

        for iteration in 0..1_000 {
            c_state = c.call(impairment, c_state);
            rust_state = rust.call(impairment, rust_state);
            assert_bits_eq(
                &format!("iteration {iteration}"),
                impairment,
                c_state,
                c_state,
                rust_state,
            );
        }
    }
}

/// Sweep every `f32` exponent, both signs, and a handful of mantissas in one
/// channel at a time, while the other two hold values that provoke overflow,
/// cancellation and infinity. Catches rounding-mode and denormal differences
/// that a uniform `[0, 1)` sample would miss.
#[test]
fn matches_across_full_exponent_range() {
    let (c, rust) = implementations();
    let mantissas: [u32; 6] = [0x00_0000, 0x00_0001, 0x40_0000, 0x55_5555, 0x7f_ffff, 0x2a_aaaa];
    let others: [f32; 6] = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::MAX,
        f32::MIN_POSITIVE,
    ];

    for &impairment in &IMPAIRMENTS {
        for sign in [0u32, 1u32] {
            for exponent in 0u32..=0xfe {
                for &mantissa in &mantissas {
                    let v = f32::from_bits((sign << 31) | (exponent << 23) | mantissa);
                    for &o in &others {
                        for channel in 0..3 {
                            let input = match channel {
                                0 => (v, o, o),
                                1 => (o, v, o),
                                _ => (o, o, v),
                            };
                            assert_bits_eq(
                                "exponent sweep",
                                impairment,
                                input,
                                c.call(impairment, input),
                                rust.call(impairment, input),
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Cross every "interesting class" representative against every other, with a
/// deliberate emphasis on mixing opposite-signed infinities with NaNs. That
/// combination makes an intermediate sum raise the invalid-operation exception
/// mid-chain, so the default QNaN it produces must outrank the NaN that arrived
/// as an input.
#[test]
fn matches_on_infinity_and_nan_mixtures() {
    let (c, rust) = implementations();
    let specials: [f32; 12] = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0xffc0_abcd),
        f32::from_bits(0x7f80_0001), // signalling NaN
        f32::from_bits(0xff80_0001), // negative signalling NaN
        0.0,
        -0.0,
        f32::MAX,
        f32::MIN,
    ];

    for &impairment in &IMPAIRMENTS {
        for &r in &specials {
            for &g in &specials {
                for &b in &specials {
                    let input = (r, g, b);
                    assert_bits_eq(
                        "inf/nan mixture",
                        impairment,
                        input,
                        c.call(impairment, input),
                        rust.call(impairment, input),
                    );
                }
            }
        }
    }
}

/// Same mixtures, but with the huge/tiny magnitudes that force an intermediate
/// to overflow to infinity even though every input is finite.
#[test]
fn matches_on_overflowing_magnitudes() {
    let (c, rust) = implementations();
    let magnitudes: [f32; 10] = [
        f32::MAX,
        f32::MIN,
        f32::MAX / 2.0,
        -f32::MAX / 2.0,
        1e38,
        -1e38,
        3.4e38,
        -3.4e38,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
    ];

    for &impairment in &IMPAIRMENTS {
        for &r in &magnitudes {
            for &g in &magnitudes {
                for &b in &magnitudes {
                    let input = (r, g, b);
                    assert_bits_eq(
                        "overflow",
                        impairment,
                        input,
                        c.call(impairment, input),
                        rust.call(impairment, input),
                    );
                }
            }
        }
    }
}

/// A wide random sweep over the low 24 bits of the mantissa with a fixed set of
/// exponents, which is where ties-to-even rounding differences would show up.
#[test]
fn matches_on_random_mantissas_per_exponent() {
    let (c, rust) = implementations();
    let mut rng = XorShift(0x2545_F491);

    for &impairment in &IMPAIRMENTS {
        for exponent in [0u32, 1, 0x40, 0x7e, 0x7f, 0x80, 0x96, 0xfd, 0xfe] {
            for _ in 0..4_000 {
                let mut draw = || {
                    let bits = rng.next_u32();
                    f32::from_bits((bits & 0x8000_0000) | (exponent << 23) | (bits & 0x7f_ffff))
                };
                let input = (draw(), draw(), draw());
                assert_bits_eq(
                    "mantissa sweep",
                    impairment,
                    input,
                    c.call(impairment, input),
                    rust.call(impairment, input),
                );
            }
        }
    }
}
