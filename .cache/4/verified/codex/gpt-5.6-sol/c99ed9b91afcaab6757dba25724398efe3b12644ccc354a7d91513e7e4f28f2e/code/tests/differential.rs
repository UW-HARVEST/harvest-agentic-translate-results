use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CbRgb255 {
    r: u8,
    g: u8,
    b: u8,
}

type Tritanopia = unsafe extern "C" fn(CbRgb255) -> CbRgb255;

const EXPECTED_MASK_COUNTS: &[(u8, u64)] = &[
    (0x00, 1_295),
    (0x01, 15),
    (0x02, 59),
    (0x04, 2_398),
    (0x05, 610),
    (0x08, 36),
    (0x09, 29_630),
    (0x0a, 7),
    (0x0b, 1_470),
    (0x0d, 52_800),
    (0x32, 2_290),
    (0x34, 27_247),
    (0x35, 111_560),
    (0x36, 344_182),
    (0x37, 1_306_364),
    (0x3a, 27_289),
    (0x3b, 658_805),
    (0x3d, 495_305),
    (0x3e, 316_093),
    (0x3f, 13_399_761),
];

fn library_path(environment_variable: &str, relative_default: &str) -> PathBuf {
    std::env::var_os(environment_variable)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative_default))
}

fn remove_gamma(value: u8) -> f32 {
    let normalized = f32::from(value) / 255.0_f32;
    if f64::from(normalized) > 0.04045 {
        (((f64::from(normalized) + 0.055) / 1.055).powf(2.4)) as f32
    } else {
        (f64::from(normalized) / 12.92) as f32
    }
}

fn branch_mask(rgb: CbRgb255, linear: &[f32; 256]) -> u8 {
    let r = linear[usize::from(rgb.r)];
    let g = linear[usize::from(rgb.g)];
    let b = linear[usize::from(rgb.b)];

    let transformed_r = r + 0.12739886310880_f32 * g - 0.12739886341072_f32 * b;
    let transformed_g = -4.486E-11_f32 * r + 0.87390929928361_f32 * g + 0.12609070101523_f32 * b;
    let transformed_b = 3.1113E-10_f32 * r + 0.87390929725848_f32 * g + 0.12609070067115_f32 * b;
    let threshold = 0.00313080495356037151702786377709;

    u8::from(rgb.r >= 11)
        | (u8::from(rgb.g >= 11) << 1)
        | (u8::from(rgb.b >= 11) << 2)
        | (u8::from(f64::from(transformed_r) > threshold) << 3)
        | (u8::from(f64::from(transformed_g) > threshold) << 4)
        | (u8::from(f64::from(transformed_b) > threshold) << 5)
}

#[test]
fn complete_domain_matches_through_shared_library_ffi() {
    let c_path = library_path("TRITANOPIA_C_LIB", "c_src/build/libtranslated_rust.so");
    let rust_path = library_path("TRITANOPIA_RUST_LIB", "target/release/libtritanopia_lib.so");
    assert!(
        c_path.is_file(),
        "missing C shared library: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {}",
        rust_path.display()
    );

    let c_library = unsafe { Library::new(&c_path) }.expect("load C shared library");
    let rust_library = unsafe { Library::new(&rust_path) }.expect("load Rust shared library");
    let c_tritanopia: Symbol<'_, Tritanopia> =
        unsafe { c_library.get(b"tritanopia\0") }.expect("resolve C tritanopia");
    let rust_tritanopia: Symbol<'_, Tritanopia> =
        unsafe { rust_library.get(b"tritanopia\0") }.expect("resolve Rust tritanopia");

    assert_eq!(std::mem::size_of::<CbRgb255>(), 3);
    assert_eq!(std::mem::align_of::<CbRgb255>(), 1);

    let linear = std::array::from_fn(|value| remove_gamma(value as u8));
    let mut mask_counts = [0_u64; 64];

    // This odd multiplier produces a bijection modulo 2^24. The fixed offset
    // makes the exhaustive traversal reproducibly pseudorandom.
    for index in 0_u32..=0x00ff_ffff {
        let packed = index.wrapping_mul(0x5bd1_e995).wrapping_add(0x006d_2b79) & 0x00ff_ffff;
        let input = CbRgb255 {
            r: packed as u8,
            g: (packed >> 8) as u8,
            b: (packed >> 16) as u8,
        };

        let c_output = unsafe { c_tritanopia(input) };
        let rust_output = unsafe { rust_tritanopia(input) };
        assert_eq!(
            rust_output, c_output,
            "output mismatch for input ({},{},{})",
            input.r, input.g, input.b
        );

        mask_counts[usize::from(branch_mask(input, &linear))] += 1;
    }

    let actual_mask_counts: Vec<(u8, u64)> = mask_counts
        .iter()
        .enumerate()
        .filter(|(_, count)| **count != 0)
        .map(|(mask, count)| (mask as u8, *count))
        .collect();
    assert_eq!(actual_mask_counts, EXPECTED_MASK_COUNTS);
    assert_eq!(mask_counts.iter().sum::<u64>(), 256_u64.pow(3));
}
