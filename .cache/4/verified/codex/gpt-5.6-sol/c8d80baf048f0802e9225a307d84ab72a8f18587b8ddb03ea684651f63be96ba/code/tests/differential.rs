use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

type Half2Float = unsafe extern "C" fn(u16) -> f32;

struct Api {
    half2float: Half2Float,
    _library: Library,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let half2float = {
            let symbol: Symbol<Half2Float> = unsafe { library.get(b"half2float\0") }
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to resolve half2float in {}: {error}",
                        path.display()
                    )
                });
            *symbol
        };

        Self {
            half2float,
            _library: library,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn target_dir() -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target"))
}

fn rust_library_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    target_dir().join(profile).join("libhalf2float_lib.so")
}

fn load_apis() -> (Api, Api) {
    let c_path = manifest_dir()
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so");
    let rust_path = rust_library_path();

    assert!(
        c_path.is_file(),
        "C shared library is missing at {}; build it with CMake first",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "Rust shared library is missing at {}; build the cdylib first",
        rust_path.display()
    );

    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn assert_match(c: &Api, rust: &Api, input: u16) {
    let c_bits = unsafe { (c.half2float)(input) }.to_bits();
    let rust_bits = unsafe { (rust.half2float)(input) }.to_bits();
    assert_eq!(
        rust_bits, c_bits,
        "half2float({input:#06x}): Rust {rust_bits:#010x}, C {c_bits:#010x}"
    );
}

fn random_u16(state: &mut u64) -> u16 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 16) as u16
}

#[test]
fn configuration_surface_matches_with_fixed_seed() {
    let (c, rust) = load_apis();
    let mut state = 0x4d59_5df4_d0f3_3173;

    // CONFIGS.md rows 1, 4, 6, and 9 are singleton classes.
    for input in [0x0000, 0x7c00, 0x8000, 0xfc00] {
        assert_match(&c, &rust, input);
    }

    // Rows 2 and 7: positive and negative subnormals.
    for sign in [0x0000, 0x8000] {
        for _ in 0..4096 {
            let fraction = 1 + random_u16(&mut state) % 0x03ff;
            assert_match(&c, &rust, sign | fraction);
        }
    }

    // Rows 3 and 8: positive and negative normals.
    for sign in [0x0000, 0x8000] {
        for _ in 0..4096 {
            let exponent = 1 + random_u16(&mut state) % 30;
            let fraction = random_u16(&mut state) & 0x03ff;
            assert_match(&c, &rust, sign | (exponent << 10) | fraction);
        }
    }

    // Rows 5 and 10: positive and negative NaNs.
    for sign in [0x0000, 0x8000] {
        for _ in 0..4096 {
            let fraction = 1 + random_u16(&mut state) % 0x03ff;
            assert_match(&c, &rust, sign | 0x7c00 | fraction);
        }
    }
}

#[test]
fn complete_ffi_domain_matches_byte_for_byte() {
    let (c, rust) = load_apis();

    for input in u16::MIN..=u16::MAX {
        assert_match(&c, &rust, input);
    }
}

#[test]
fn ffi_domain_boundaries_match() {
    let (c, rust) = load_apis();

    for input in [u16::MIN, u16::MIN + 1, u16::MAX - 1, u16::MAX] {
        assert_match(&c, &rust, input);
    }
}
