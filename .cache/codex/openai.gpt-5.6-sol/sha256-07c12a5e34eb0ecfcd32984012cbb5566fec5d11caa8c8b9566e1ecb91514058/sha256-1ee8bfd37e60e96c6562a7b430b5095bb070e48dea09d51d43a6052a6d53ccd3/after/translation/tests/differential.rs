use libloading::Library;
use std::path::{Path, PathBuf};

type Float2Half = unsafe extern "C" fn(f32) -> u16;

struct ApiPair {
    _c_library: Library,
    _rust_library: Library,
    c_float2half: Float2Half,
    rust_float2half: Float2Half,
}

impl ApiPair {
    fn load() -> Self {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_library_path = find_c_library(&manifest_dir.join("../c_src/build"));
        let profile_dir = std::env::current_exe()
            .expect("test executable path")
            .parent()
            .expect("test executable directory")
            .parent()
            .expect("Cargo profile directory")
            .to_path_buf();
        let profile_library_path = profile_dir.join("libfloat2half_lib.so");
        let rust_library_path = if profile_library_path.is_file() {
            profile_library_path
        } else {
            manifest_dir.join("target/release/libfloat2half_lib.so")
        };

        assert!(
            rust_library_path.is_file(),
            "Rust cdylib does not exist at {}",
            rust_library_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_library_path).unwrap_or_else(|error| {
                panic!(
                    "failed to load C library {}: {error}",
                    c_library_path.display()
                )
            });
            let rust_library = Library::new(&rust_library_path).unwrap_or_else(|error| {
                panic!(
                    "failed to load Rust library {}: {error}",
                    rust_library_path.display()
                )
            });
            let c_float2half = *c_library
                .get::<Float2Half>(b"float2half\0")
                .expect("C float2half export");
            let rust_float2half = *rust_library
                .get::<Float2Half>(b"float2half\0")
                .expect("Rust float2half export");

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_float2half,
                rust_float2half,
            }
        }
    }

    fn assert_match(&self, bits: u32) {
        let input = f32::from_bits(bits);
        let c_result = unsafe { (self.c_float2half)(input) };
        let rust_result = unsafe { (self.rust_float2half)(input) };

        assert_eq!(
            c_result.to_ne_bytes(),
            rust_result.to_ne_bytes(),
            "mismatch for float bits 0x{bits:08x}: C=0x{c_result:04x}, Rust=0x{rust_result:04x}"
        );
    }
}

fn find_c_library(build_dir: &Path) -> PathBuf {
    let mut libraries: Vec<_> = std::fs::read_dir(build_dir)
        .unwrap_or_else(|error| {
            panic!(
                "failed to read C build directory {}: {error}",
                build_dir.display()
            )
        })
        .map(|entry| entry.expect("C build directory entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    libraries.sort();

    assert_eq!(
        libraries.len(),
        1,
        "expected exactly one C shared library in {}",
        build_dir.display()
    );
    libraries.pop().unwrap()
}

fn raw_bits(sign: bool, exponent: u8, mantissa: u32) -> u32 {
    ((sign as u32) << 31) | ((exponent as u32) << 23) | (mantissa & 0x007f_ffff)
}

fn next_random(state: &mut u64) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 16) as u32
}

fn check_exponent_range(sign: bool, first: u8, last: u8, seed: u64) {
    const BOUNDARY_MANTISSAS: [u32; 12] = [
        0x000000, 0x000001, 0x000fff, 0x001000, 0x001fff, 0x002000, 0x3fffff, 0x400000, 0x7fdfff,
        0x7fe000, 0x7ffffe, 0x7fffff,
    ];

    let api = ApiPair::load();
    let mut random_state = seed;

    for exponent in first..=last {
        for mantissa in BOUNDARY_MANTISSAS {
            api.assert_match(raw_bits(sign, exponent, mantissa));
        }
        for _ in 0..256 {
            let mantissa = next_random(&mut random_state) & 0x007f_ffff;
            api.assert_match(raw_bits(sign, exponent, mantissa));
        }
    }
}

fn check_infinity(sign: bool) {
    ApiPair::load().assert_match(raw_bits(sign, 255, 0));
}

fn check_nan_payloads(sign: bool, seed: u64) {
    const BOUNDARY_PAYLOADS: [u32; 10] = [
        0x000001, 0x000fff, 0x001000, 0x001fff, 0x002000, 0x3fffff, 0x400000, 0x7fe000, 0x7ffffe,
        0x7fffff,
    ];

    let api = ApiPair::load();
    let mut random_state = seed;

    for payload in BOUNDARY_PAYLOADS {
        api.assert_match(raw_bits(sign, 255, payload));
    }
    for _ in 0..4096 {
        let payload = (next_random(&mut random_state) & 0x007f_ffff).max(1);
        api.assert_match(raw_bits(sign, 255, payload));
    }
}

#[test]
fn config_01_positive_exponent_zero() {
    check_exponent_range(false, 0, 0, 0x01a2_b3c4_d5e6_f701);
}

#[test]
fn config_02_negative_exponent_zero() {
    check_exponent_range(true, 0, 0, 0x02a2_b3c4_d5e6_f702);
}

#[test]
fn config_03_positive_below_half_subnormal() {
    check_exponent_range(false, 1, 102, 0x03a2_b3c4_d5e6_f703);
}

#[test]
fn config_04_negative_below_half_subnormal() {
    check_exponent_range(true, 1, 102, 0x04a2_b3c4_d5e6_f704);
}

#[test]
fn config_05_positive_half_subnormal() {
    check_exponent_range(false, 103, 112, 0x05a2_b3c4_d5e6_f705);
}

#[test]
fn config_06_negative_half_subnormal() {
    check_exponent_range(true, 103, 112, 0x06a2_b3c4_d5e6_f706);
}

#[test]
fn config_07_positive_half_normal() {
    check_exponent_range(false, 113, 142, 0x07a2_b3c4_d5e6_f707);
}

#[test]
fn config_08_negative_half_normal() {
    check_exponent_range(true, 113, 142, 0x08a2_b3c4_d5e6_f708);
}

#[test]
fn config_09_positive_finite_overflow() {
    check_exponent_range(false, 143, 254, 0x09a2_b3c4_d5e6_f709);
}

#[test]
fn config_10_negative_finite_overflow() {
    check_exponent_range(true, 143, 254, 0x10a2_b3c4_d5e6_f710);
}

#[test]
fn config_11_positive_infinity() {
    check_infinity(false);
}

#[test]
fn config_12_negative_infinity() {
    check_infinity(true);
}

#[test]
fn config_13_positive_nan_payloads() {
    check_nan_payloads(false, 0x13a2_b3c4_d5e6_f713);
}

#[test]
fn config_14_negative_nan_payloads() {
    check_nan_payloads(true, 0x14a2_b3c4_d5e6_f714);
}
