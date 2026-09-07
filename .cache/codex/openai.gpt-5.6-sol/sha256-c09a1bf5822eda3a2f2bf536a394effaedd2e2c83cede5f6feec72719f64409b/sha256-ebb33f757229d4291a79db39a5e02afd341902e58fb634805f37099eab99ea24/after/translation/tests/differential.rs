use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

type LdexpQ2 = unsafe extern "C" fn(f32, i32) -> f32;

const INTERESTING_FLOAT_BITS: &[u32] = &[
    0x0000_0000, // +0
    0x8000_0000, // -0
    0x0000_0001, // smallest positive subnormal
    0x8000_0001, // smallest negative subnormal
    0x007f_ffff, // largest positive subnormal
    0x807f_ffff, // largest negative subnormal
    0x0080_0000, // smallest positive normal
    0x8080_0000, // smallest negative normal
    0x3f80_0000, // 1
    0xbf80_0000, // -1
    0x7f7f_ffff, // largest positive finite
    0xff7f_ffff, // largest negative finite
    0x7f80_0000, // +infinity
    0xff80_0000, // -infinity
    0x7fc0_0000, // quiet NaN
    0xffc1_2345, // negative quiet NaN with payload
    0x7f80_0001, // signaling NaN with payload
    0xff80_0001, // negative signaling NaN with payload
];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("translation crate must have a parent")
        .join("c_src/build/libharvest-work-54f7Vc.so")
}

fn rust_library_path() -> PathBuf {
    let target = manifest_dir().join("target");
    let preferred_profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let preferred = target.join(preferred_profile).join("libldexp_q2_lib.so");
    if preferred.is_file() {
        return preferred;
    }

    let fallback_profile = if preferred_profile == "debug" {
        "release"
    } else {
        "debug"
    };
    target.join(fallback_profile).join("libldexp_q2_lib.so")
}

fn assert_library_exists(path: &Path, implementation: &str) {
    assert!(
        path.is_file(),
        "{implementation} shared library does not exist at {}",
        path.display()
    );
}

fn with_implementations(test: impl FnOnce(LdexpQ2, LdexpQ2)) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert_library_exists(&c_path, "C");
    assert_library_exists(&rust_path, "Rust");

    // Keep both Library values alive for the entire lifetime of the symbols.
    let c_library = unsafe { Library::new(&c_path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
    let rust_library = unsafe { Library::new(&rust_path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
    let c_symbol: Symbol<LdexpQ2> = unsafe { c_library.get(b"ldexp_q2\0") }
        .unwrap_or_else(|error| panic!("C ldexp_q2 export is unavailable: {error}"));
    let rust_symbol: Symbol<LdexpQ2> = unsafe { rust_library.get(b"ldexp_q2\0") }
        .unwrap_or_else(|error| panic!("Rust ldexp_q2 export is unavailable: {error}"));

    test(*c_symbol, *rust_symbol);
}

fn next_random_u32(state: &mut u64) -> u32 {
    // Fixed-seed xorshift64* generator: deterministic and dependency-free.
    let mut value = *state;
    value ^= value >> 12;
    value ^= value << 25;
    value ^= value >> 27;
    *state = value;
    (value.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
}

#[track_caller]
fn compare_one(c_fn: LdexpQ2, rust_fn: LdexpQ2, y_bits: u32, exp_q2: i32) {
    let y = f32::from_bits(y_bits);
    let c_result = unsafe { c_fn(y, exp_q2) };
    let rust_result = unsafe { rust_fn(y, exp_q2) };
    assert_eq!(
        c_result.to_bits(),
        rust_result.to_bits(),
        "mismatch for y_bits=0x{y_bits:08x}, exp_q2={exp_q2}: \
         C=0x{:08x}, Rust=0x{:08x}",
        c_result.to_bits(),
        rust_result.to_bits()
    );
}

fn compare_exponents(row: u64, exponents: &[i32], random_samples_per_exponent: usize) {
    with_implementations(|c_fn, rust_fn| {
        let mut state = 0xd1ff_3a5e_0000_0000_u64 ^ row;
        for &exp_q2 in exponents {
            for &y_bits in INTERESTING_FLOAT_BITS {
                compare_one(c_fn, rust_fn, y_bits, exp_q2);
            }
            for _ in 0..random_samples_per_exponent {
                compare_one(c_fn, rust_fn, next_random_u32(&mut state), exp_q2);
            }
        }
    });
}

fn one_pass_exponents(selector: i32) -> Vec<i32> {
    (1..120).filter(|exp_q2| exp_q2 & 3 == selector).collect()
}

fn multi_pass_exponents(selector: i32) -> Vec<i32> {
    let quotients = [1_i32, 2, 3, 7, 31, 257];
    let mut exponents = Vec::new();
    for remainder in 1_i32..120 {
        if remainder & 3 == selector {
            let quotient = quotients[(remainder as usize) % quotients.len()];
            exponents.push(quotient * 120 + remainder);
        }
    }
    exponents
}

#[test]
fn config_01_negative_exponents() {
    compare_exponents(
        1,
        &[
            i32::MIN,
            i32::MIN + 1,
            -1_000_000,
            -129,
            -128,
            -125,
            -124,
            -123,
            -121,
            -120,
            -119,
            -65,
            -64,
            -33,
            -32,
            -31,
            -5,
            -4,
            -3,
            -2,
            -1,
        ],
        256,
    );
}

#[test]
fn config_02_zero_exponent() {
    compare_exponents(2, &[0], 8_192);
}

#[test]
fn config_03_one_pass_selector_0() {
    compare_exponents(3, &one_pass_exponents(0), 256);
}

#[test]
fn config_04_one_pass_selector_1() {
    compare_exponents(4, &one_pass_exponents(1), 256);
}

#[test]
fn config_05_one_pass_selector_2() {
    compare_exponents(5, &one_pass_exponents(2), 256);
}

#[test]
fn config_06_one_pass_selector_3() {
    compare_exponents(6, &one_pass_exponents(3), 256);
}

#[test]
fn config_07_exact_minimum_cap_boundary() {
    compare_exponents(7, &[120], 8_192);
}

#[test]
fn config_08_multi_pass_exact_multiples_of_120() {
    compare_exponents(8, &[240, 360, 480, 600, 1_200, 12_000, 120_000], 512);
}

#[test]
fn config_09_multi_pass_final_selector_0() {
    compare_exponents(9, &multi_pass_exponents(0), 256);
}

#[test]
fn config_10_multi_pass_final_selector_1() {
    compare_exponents(10, &multi_pass_exponents(1), 256);
}

#[test]
fn config_11_multi_pass_final_selector_2() {
    compare_exponents(11, &multi_pass_exponents(2), 256);
}

#[test]
fn config_12_multi_pass_final_selector_3() {
    compare_exponents(12, &multi_pass_exponents(3), 256);

    // INT_MAX has remainder 7 after repeated 120-sized chunks, selecting
    // g_expfrac[3]. Keep this boundary sample small because C performs
    // 17,895,697 loop iterations before returning.
    compare_exponents(12_000, &[i32::MAX], 1);
}

#[test]
fn ffi_symbol_can_be_resolved_from_both_shared_libraries() {
    with_implementations(|_, _| {});
}
