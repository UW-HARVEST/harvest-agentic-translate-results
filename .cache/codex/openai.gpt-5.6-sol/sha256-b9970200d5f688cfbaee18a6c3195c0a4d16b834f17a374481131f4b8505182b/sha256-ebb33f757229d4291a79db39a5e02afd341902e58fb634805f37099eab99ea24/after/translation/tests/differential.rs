use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

type Half2Float = unsafe extern "C" fn(u16) -> f32;

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_dir = crate_dir
            .parent()
            .expect("translation crate must have a workspace parent");
        let c_path = workspace_dir.join("c_src/build/libharvest-work-KE36At.so");
        let rust_path = crate_dir.join("target/release/libhalf2float_lib.so");

        Self {
            c: load_library(&c_path),
            rust: load_library(&rust_path),
        }
    }

    fn compare(&self, input: u16) {
        unsafe {
            let c_fn: Symbol<Half2Float> = self
                .c
                .get(b"half2float\0")
                .expect("C library must export half2float");
            let rust_fn: Symbol<Half2Float> = self
                .rust
                .get(b"half2float\0")
                .expect("Rust library must export half2float");

            let c_bits = c_fn(input).to_bits();
            let rust_bits = rust_fn(input).to_bits();
            assert_eq!(
                rust_bits, c_bits,
                "output mismatch for half bits 0x{input:04x}: C=0x{c_bits:08x}, Rust=0x{rust_bits:08x}"
            );
        }
    }
}

fn load_library(path: &Path) -> Library {
    assert!(
        path.is_file(),
        "required shared library does not exist: {}",
        path.display()
    );
    unsafe { Library::new(path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()))
}

fn randomized_inputs(seed: u64, count: usize, make_input: impl Fn(u16) -> u16) -> Vec<u16> {
    let mut state = seed;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        values.push(make_input(state as u16));
    }
    values
}

fn compare_many(values: impl IntoIterator<Item = u16>) {
    let libraries = Libraries::load();
    for input in values {
        libraries.compare(input);
    }
}

#[test]
fn config_01_positive_zero() {
    compare_many(std::iter::repeat_n(0x0000, 256));
}

#[test]
fn config_02_positive_subnormal() {
    compare_many(randomized_inputs(0x8a5c_19d3_2047_61ef, 4096, |value| {
        (value % 1023) + 1
    }));
}

#[test]
fn config_03_positive_normal() {
    compare_many(randomized_inputs(0xd177_32aa_940e_4c6b, 8192, |value| {
        let exponent = ((value >> 10) % 30) + 1;
        (exponent << 10) | (value & 0x03ff)
    }));
}

#[test]
fn config_04_positive_infinity() {
    compare_many(std::iter::repeat_n(0x7c00, 256));
}

#[test]
fn config_05_positive_nan() {
    compare_many(randomized_inputs(0x7b9e_a402_18c5_f36d, 4096, |value| {
        0x7c00 | ((value % 1023) + 1)
    }));
}

#[test]
fn config_06_negative_zero() {
    compare_many(std::iter::repeat_n(0x8000, 256));
}

#[test]
fn config_07_negative_subnormal() {
    compare_many(randomized_inputs(0xec45_0d17_b8a2_693f, 4096, |value| {
        0x8000 | ((value % 1023) + 1)
    }));
}

#[test]
fn config_08_negative_normal() {
    compare_many(randomized_inputs(0x312f_c896_5da0_b47e, 8192, |value| {
        let exponent = ((value >> 10) % 30) + 1;
        0x8000 | (exponent << 10) | (value & 0x03ff)
    }));
}

#[test]
fn config_09_negative_infinity() {
    compare_many(std::iter::repeat_n(0xfc00, 256));
}

#[test]
fn config_10_negative_nan() {
    compare_many(randomized_inputs(0x56bd_71e3_0ac9_842f, 4096, |value| {
        0xfc00 | ((value % 1023) + 1)
    }));
}

#[test]
fn exhaustive_all_u16_inputs_match_byte_for_byte() {
    compare_many(u16::MIN..=u16::MAX);
}

#[test]
fn error_surface_is_empty_because_every_u16_is_valid() {
    // This by-value API has no pointers, lengths, enums, or invalid bit patterns.
    // Calling the complete input domain proves that neither implementation has an
    // additional rejection boundary hidden outside the documented surface.
    compare_many(u16::MIN..=u16::MAX);
}
