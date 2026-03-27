use libloading::{Library, Symbol};
use contrast_ratio_lib::{cb_rgb_255, contrast_ratio};

fn load_c_lib() -> Library {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = format!("{}/c_src/build/libcontrast_ratio_lib.so", manifest);
    unsafe { Library::new(&path).expect("Failed to load C library") }
}

fn c_contrast_ratio(lib: &Library, a: cb_rgb_255, b: cb_rgb_255) -> f32 {
    unsafe {
        let func: Symbol<unsafe extern "C" fn(cb_rgb_255, cb_rgb_255) -> f32> =
            lib.get(b"contrast_ratio").unwrap();
        func(a, b)
    }
}

#[test]
fn test_contrast_ratio_byte_identical() {
    let lib = load_c_lib();

    let test_cases: Vec<(cb_rgb_255, cb_rgb_255)> = vec![
        // Black vs White
        (cb_rgb_255 { R: 0, G: 0, B: 0 }, cb_rgb_255 { R: 255, G: 255, B: 255 }),
        // Same color
        (cb_rgb_255 { R: 128, G: 128, B: 128 }, cb_rgb_255 { R: 128, G: 128, B: 128 }),
        // Red vs Blue
        (cb_rgb_255 { R: 255, G: 0, B: 0 }, cb_rgb_255 { R: 0, G: 0, B: 255 }),
        // Near-threshold values (0.04045 * 255 ≈ 10.3)
        (cb_rgb_255 { R: 10, G: 10, B: 10 }, cb_rgb_255 { R: 11, G: 11, B: 11 }),
        // All zeros
        (cb_rgb_255 { R: 0, G: 0, B: 0 }, cb_rgb_255 { R: 0, G: 0, B: 0 }),
        // Arbitrary colors
        (cb_rgb_255 { R: 42, G: 170, B: 200 }, cb_rgb_255 { R: 200, G: 50, B: 100 }),
        (cb_rgb_255 { R: 1, G: 1, B: 1 }, cb_rgb_255 { R: 254, G: 254, B: 254 }),
        // Edge: one channel max, others zero
        (cb_rgb_255 { R: 255, G: 0, B: 0 }, cb_rgb_255 { R: 0, G: 255, B: 0 }),
        (cb_rgb_255 { R: 0, G: 255, B: 0 }, cb_rgb_255 { R: 0, G: 0, B: 255 }),
    ];

    for (i, (a, b)) in test_cases.iter().enumerate() {
        let a_copy = cb_rgb_255 { R: a.R, G: a.G, B: a.B };
        let b_copy = cb_rgb_255 { R: b.R, G: b.G, B: b.B };
        let a_copy2 = cb_rgb_255 { R: a.R, G: a.G, B: a.B };
        let b_copy2 = cb_rgb_255 { R: b.R, G: b.G, B: b.B };

        let c_result = c_contrast_ratio(&lib, a_copy, b_copy);
        let rust_result = contrast_ratio(a_copy2, b_copy2);

        let c_bits = c_result.to_bits();
        let rust_bits = rust_result.to_bits();

        assert_eq!(
            c_bits, rust_bits,
            "Case {i}: C={c_result} (0x{c_bits:08x}) != Rust={rust_result} (0x{rust_bits:08x}) \
             for A=({},{},{}) B=({},{},{})",
            a.R, a.G, a.B, b.R, b.G, b.B
        );
    }
}

/// Exhaustive sweep: test all R values with fixed G,B to catch any edge cases
#[test]
fn test_contrast_ratio_sweep() {
    let lib = load_c_lib();
    let b = cb_rgb_255 { R: 128, G: 64, B: 200 };

    for r in 0..=255u8 {
        let a = cb_rgb_255 { R: r, G: 100, B: 50 };
        let a2 = cb_rgb_255 { R: r, G: 100, B: 50 };
        let b1 = cb_rgb_255 { R: b.R, G: b.G, B: b.B };
        let b2 = cb_rgb_255 { R: b.R, G: b.G, B: b.B };

        let c_result = c_contrast_ratio(&lib, a, b1);
        let rust_result = contrast_ratio(a2, b2);

        assert_eq!(
            c_result.to_bits(),
            rust_result.to_bits(),
            "Sweep R={r}: C={c_result} != Rust={rust_result}"
        );
    }
}
