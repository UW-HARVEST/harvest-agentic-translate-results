use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

#[test]
fn test_float2half_comprehensive() {
    let lib = unsafe { Library::new(c_lib_path()) }.expect("Failed to load C .so");
    let c_float2half: Symbol<unsafe extern "C" fn(f32) -> u16> =
        unsafe { lib.get(b"float2half") }.expect("Failed to find float2half");

    // Test values covering: zero, subnormals, normals, large, inf, nan, negatives
    let test_values: Vec<f32> = {
        let mut v = vec![
            0.0f32, -0.0, 1.0, -1.0, 0.5, -0.5, 2.0, -2.0,
            0.333333, 65504.0, // max half
            f32::INFINITY, f32::NEG_INFINITY,
            f32::NAN,
            1.0e-8, // very small
            1.0e38, // very large
            0.00006103515625, // smallest normal half
            0.000000059604645, // smallest subnormal half
        ];
        // Sweep all 512 exponent+sign combos with a representative mantissa
        for j in 0u32..512 {
            let bits = j << 23;
            v.push(f32::from_bits(bits));
            v.push(f32::from_bits(bits | 0x007FFFFF)); // max mantissa
            v.push(f32::from_bits(bits | 0x00400000)); // mid mantissa
        }
        v
    };

    let mut mismatches = 0;
    for &val in &test_values {
        let c_result = unsafe { c_float2half(val) };
        let rust_result = float2half_lib::float2half(val);
        if c_result != rust_result {
            eprintln!(
                "MISMATCH: float2half({val}) [bits=0x{:08x}] C={c_result:#06x} Rust={rust_result:#06x}",
                val.to_bits()
            );
            mismatches += 1;
        }
    }
    assert_eq!(mismatches, 0, "{mismatches} mismatches found");
}
