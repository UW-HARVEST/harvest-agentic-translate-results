use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

type Tritanopia = unsafe extern "C" fn(Rgb) -> Rgb;

const DOMAIN_SIZE: u32 = 1 << 24;
const DOMAIN_MASK: u32 = DOMAIN_SIZE - 1;
const PERMUTATION_MULTIPLIER: u32 = 0x00c0_ffee;
const PERMUTATION_SEED: u32 = 0x005e_ed5e;

const EXPECTED_SIGNATURE_COUNTS: [u64; 64] = [
    1_295, 0, 0, 0, 36, 0, 0, 0, // input mask 000
    2_398, 0, 0, 27_247, 0, 0, 0, 0, // input mask 001
    59, 0, 0, 2_290, 7, 0, 0, 27_289, // input mask 010
    0, 0, 0, 344_182, 0, 0, 0, 316_093, // input mask 011
    15, 0, 0, 0, 29_630, 0, 0, 0, // input mask 100
    610, 0, 0, 111_560, 52_800, 0, 0, 495_305, // input mask 101
    0, 0, 0, 0, 1_470, 0, 0, 658_805, // input mask 110
    0, 0, 0, 1_306_364, 0, 0, 0, 13_399_761, // input mask 111
];

fn c_library_path() -> PathBuf {
    let build_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    let mut candidates: Vec<_> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build_dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("lib") && name.ends_with(".so"))
        })
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared library in {}, found {candidates:?}",
        build_dir.display()
    );
    candidates.pop().unwrap()
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libtritanopia_lib.so")
}

fn remove_gamma(value: f32) -> f32 {
    if f64::from(value) > 0.04045 {
        (((f64::from(value) + 0.055) / 1.055).powf(2.4)) as f32
    } else {
        (f64::from(value) / 12.92) as f32
    }
}

fn branch_signature(input: Rgb) -> usize {
    let input_mask =
        usize::from(input.r > 10) * 4 + usize::from(input.g > 10) * 2 + usize::from(input.b > 10);

    let r = remove_gamma(input.r as f32 / 255.0_f32);
    let g = remove_gamma(input.g as f32 / 255.0_f32);
    let b = remove_gamma(input.b as f32 / 255.0_f32);

    let transformed_r = r + 0.12739886310880_f32 * g - 0.12739886341072_f32 * b;
    let transformed_g = -4.486E-11_f32 * r + 0.87390929928361_f32 * g + 0.12609070101523_f32 * b;
    let transformed_b = 3.1113E-10_f32 * r + 0.87390929725848_f32 * g + 0.12609070067115_f32 * b;
    let threshold = 0.00313080495356037151702786377709_f64;
    let output_mask = usize::from(f64::from(transformed_r) > threshold) * 4
        + usize::from(f64::from(transformed_g) > threshold) * 2
        + usize::from(f64::from(transformed_b) > threshold);

    input_mask * 8 + output_mask
}

#[test]
fn all_valid_inputs_match_through_the_dynamic_ffi_surface() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(rust_path.is_file(), "missing {}", rust_path.display());

    // Keep both libraries alive until after their symbols are no longer used.
    let c_library = unsafe { Library::new(&c_path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
    let rust_library = unsafe { Library::new(&rust_path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
    let c_tritanopia: Symbol<'_, Tritanopia> = unsafe { c_library.get(b"tritanopia\0") }
        .unwrap_or_else(|error| panic!("C tritanopia export missing: {error}"));
    let rust_tritanopia: Symbol<'_, Tritanopia> = unsafe { rust_library.get(b"tritanopia\0") }
        .unwrap_or_else(|error| panic!("Rust tritanopia export missing: {error}"));

    let mut signature_counts = [0_u64; 64];
    for index in 0..DOMAIN_SIZE {
        // An odd affine multiplier is a bijection modulo 2^24, so this is a
        // fixed-seed randomized traversal of every possible public input.
        let packed = index
            .wrapping_mul(PERMUTATION_MULTIPLIER | 1)
            .wrapping_add(PERMUTATION_SEED)
            & DOMAIN_MASK;
        let input = Rgb {
            r: (packed >> 16) as u8,
            g: (packed >> 8) as u8,
            b: packed as u8,
        };
        signature_counts[branch_signature(input)] += 1;

        let expected = unsafe { c_tritanopia(input) };
        let actual = unsafe { rust_tritanopia(input) };
        assert_eq!(
            actual, expected,
            "dynamic FFI mismatch for input ({},{},{})",
            input.r, input.g, input.b
        );
    }

    assert_eq!(
        signature_counts, EXPECTED_SIGNATURE_COUNTS,
        "the test classifier no longer matches the C-derived CONFIGS.md surface"
    );
}
