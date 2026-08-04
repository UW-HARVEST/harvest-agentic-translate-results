use libloading::{Library, Symbol};
use std::path::PathBuf;

type Float2HalfFn = unsafe extern "C" fn(f32) -> u16;

fn load_libs() -> (Library, Library) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_lib = manifest.join("c_src/build/libtranslated_rust.so");
    let rust_lib = manifest.join("target/debug/libfloat2half_lib.so");
    unsafe {
        (
            Library::new(&c_lib).expect("failed to load C .so"),
            Library::new(&rust_lib).expect("failed to load Rust .so"),
        )
    }
}

#[test]
fn test_float2half_comprehensive() {
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<Float2HalfFn> = unsafe { c_lib.get(b"float2half").unwrap() };
    let r_fn: Symbol<Float2HalfFn> = unsafe { rust_lib.get(b"float2half").unwrap() };

    // Specific interesting values
    let special: Vec<f32> = vec![
        0.0, -0.0, 1.0, -1.0, 0.5, -0.5,
        f32::INFINITY, f32::NEG_INFINITY,
        f32::NAN,
        f32::MIN_POSITIVE, -f32::MIN_POSITIVE,
        f32::MAX, f32::MIN,
        // Subnormal half-precision boundary
        5.96e-8, -5.96e-8,
        // Half-precision max normal
        65504.0, -65504.0,
        // Just above half max -> should clamp to inf
        65536.0, -65536.0,
        // Small subnormals
        1e-40, -1e-40,
        // Values near boundaries
        0.333333, 2.0, 3.0, 100.0, 0.1, 0.01,
    ];

    let mut mismatches = 0u64;
    for &val in &special {
        let c_result = unsafe { c_fn(val) };
        let r_result = unsafe { r_fn(val) };
        assert_eq!(
            c_result, r_result,
            "MISMATCH for f32 bits={:#010x} ({}): C={:#06x}, Rust={:#06x}",
            val.to_bits(), val, c_result, r_result
        );
    }

    // Exhaustive: test every possible float bit pattern via exponent sweep
    // Test all 512 exponent+sign combos with several mantissa values each
    for j in 0u32..512 {
        let sign_exp = if j < 256 {
            j << 23
        } else {
            (1 << 31) | ((j - 256) << 23)
        };
        // Test mantissa = 0, max, and a few in between
        for &mantissa in &[0u32, 0x007fffff, 0x00400000, 0x00000001, 0x003fffff] {
            let bits = sign_exp | mantissa;
            let val = f32::from_bits(bits);
            let c_result = unsafe { c_fn(val) };
            let r_result = unsafe { r_fn(val) };
            if c_result != r_result {
                mismatches += 1;
                panic!(
                    "MISMATCH for f32 bits={:#010x}: C={:#06x}, Rust={:#06x}",
                    bits, c_result, r_result
                );
            }
        }
    }

    assert_eq!(mismatches, 0, "{} mismatches found", mismatches);
}

#[test]
fn test_float2half_exhaustive_all_u32() {
    // Test ALL 2^32 float bit patterns for byte-identical results
    // This takes a while but is the definitive correctness check
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<Float2HalfFn> = unsafe { c_lib.get(b"float2half").unwrap() };
    let r_fn: Symbol<Float2HalfFn> = unsafe { rust_lib.get(b"float2half").unwrap() };

    let mut mismatches = 0u64;
    let mut first_mismatch = None;

    for bits in 0u32..=u32::MAX {
        let val = f32::from_bits(bits);
        let c_result = unsafe { c_fn(val) };
        let r_result = unsafe { r_fn(val) };
        if c_result != r_result {
            mismatches += 1;
            if first_mismatch.is_none() {
                first_mismatch = Some((bits, c_result, r_result));
            }
            if mismatches >= 10 {
                break;
            }
        }
    }

    if let Some((bits, c, r)) = first_mismatch {
        panic!(
            "{} mismatches. First: bits={:#010x}, C={:#06x}, Rust={:#06x}",
            mismatches, bits, c, r
        );
    }
}
