use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

type Half2Float = unsafe extern "C" fn(u16) -> f32;

struct Implementations {
    _c_library: Library,
    _rust_library: Library,
    c_half2float: Half2Float,
    rust_half2float: Half2Float,
}

impl Implementations {
    fn load() -> Self {
        let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = crate_root.join("../c_src/build/libharvest-work-UwAYVp.so");
        let rust_path = crate_root.join("target/release/libhalf2float_lib.so");

        assert_library_exists(&c_path);
        assert_library_exists(&rust_path);

        // SAFETY: Both paths are build artifacts controlled by this test.
        let c_library = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
        // SAFETY: Both paths are build artifacts controlled by this test.
        let rust_library = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));

        // SAFETY: The public C header declares this exact ABI and signature.
        let c_half2float = unsafe {
            let symbol: Symbol<'_, Half2Float> = c_library
                .get(b"half2float\0")
                .expect("C library is missing half2float");
            *symbol
        };
        // SAFETY: The Rust cdylib must expose the same C ABI and signature.
        let rust_half2float = unsafe {
            let symbol: Symbol<'_, Half2Float> = rust_library
                .get(b"half2float\0")
                .expect("Rust library is missing half2float");
            *symbol
        };

        Self {
            _c_library: c_library,
            _rust_library: rust_library,
            c_half2float,
            rust_half2float,
        }
    }

    fn assert_match(&self, input: u16, configuration: &str) {
        // SAFETY: A u16 is the complete valid input domain for both functions.
        let c_output = unsafe { (self.c_half2float)(input) };
        // SAFETY: A u16 is the complete valid input domain for both functions.
        let rust_output = unsafe { (self.rust_half2float)(input) };

        assert_eq!(
            c_output.to_ne_bytes(),
            rust_output.to_ne_bytes(),
            "{configuration} diverged for input 0x{input:04x}: C=0x{:08x}, Rust=0x{:08x}",
            c_output.to_bits(),
            rust_output.to_bits()
        );
    }
}

fn assert_library_exists(path: &Path) {
    assert!(
        path.is_file(),
        "missing {}; build both shared libraries before running this test",
        path.display()
    );
}

fn next_random(state: &mut u64) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 16) as u32
}

fn make_half(sign: u16, exponent: u16, fraction: u16) -> u16 {
    (sign << 15) | (exponent << 10) | fraction
}

#[test]
fn every_configuration_matches_for_fixed_seed_randomized_inputs() {
    let implementations = Implementations::load();
    let mut random_state = 0x6a09_e667_f3bc_c909;

    let singleton_configurations = [
        ("positive zero", make_half(0, 0, 0)),
        ("negative zero", make_half(1, 0, 0)),
        ("positive infinity", make_half(0, 31, 0)),
        ("negative infinity", make_half(1, 31, 0)),
    ];
    for (configuration, input) in singleton_configurations {
        implementations.assert_match(input, configuration);
    }

    for (configuration, sign) in [("positive subnormal", 0), ("negative subnormal", 1)] {
        for _ in 0..4096 {
            let fraction = (next_random(&mut random_state) % 1023 + 1) as u16;
            implementations.assert_match(make_half(sign, 0, fraction), configuration);
        }
    }

    for (configuration, sign) in [("positive normal", 0), ("negative normal", 1)] {
        for _ in 0..20_000 {
            let exponent = (next_random(&mut random_state) % 30 + 1) as u16;
            let fraction = (next_random(&mut random_state) % 1024) as u16;
            implementations.assert_match(make_half(sign, exponent, fraction), configuration);
        }
    }

    for (configuration, sign) in [("positive NaN", 0), ("negative NaN", 1)] {
        for _ in 0..4096 {
            let fraction = (next_random(&mut random_state) % 1023 + 1) as u16;
            implementations.assert_match(make_half(sign, 31, fraction), configuration);
        }
    }
}

#[test]
fn complete_u16_input_domain_matches() {
    let implementations = Implementations::load();

    for input in u16::MIN..=u16::MAX {
        implementations.assert_match(input, "complete uint16_t domain");
    }
}
