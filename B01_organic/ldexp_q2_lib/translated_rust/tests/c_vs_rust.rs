use libloading::{Library, Symbol};

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libldexp_q2_lib.so");
    unsafe { Library::new(path).expect("Failed to load C .so") }
}

#[test]
fn test_ldexp_q2_matches_c() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f32, i32) -> f32> =
        unsafe { lib.get(b"ldexp_q2").expect("symbol not found") };

    // Test cases: (y, exp_q2)
    let cases: &[(f32, i32)] = &[
        (1.0, 1),
        (1.0, 4),
        (1.0, 8),
        (1.0, 120),
        (0.5, 1),
        (0.5, 2),
        (0.5, 3),
        (0.5, 4),
        (2.0, 10),
        (1.0, 30 * 4),    // exactly the clamp boundary
        (1.0, 30 * 4 + 1), // exceeds clamp, triggers loop
        (100.0, 5),
        (0.001, 20),
        (1e10, 3),
        (1e-10, 7),
        (-1.0, 4),
        (-0.5, 8),
        (f32::MIN_POSITIVE, 1),
        (f32::MAX, 1),
    ];

    for &(y, exp_q2) in cases {
        let c_result = unsafe { c_fn(y, exp_q2) };
        let rust_result = ldexp_q2_lib::ldexp_q2(y, exp_q2);
        assert_eq!(
            c_result.to_bits(),
            rust_result.to_bits(),
            "MISMATCH for y={y}, exp_q2={exp_q2}: C={c_result} (0x{:08x}), Rust={rust_result} (0x{:08x})",
            c_result.to_bits(),
            rust_result.to_bits()
        );
    }
}
