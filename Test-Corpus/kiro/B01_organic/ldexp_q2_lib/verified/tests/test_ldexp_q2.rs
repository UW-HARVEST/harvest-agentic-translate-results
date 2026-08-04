use libloading::{Library, Symbol};
use std::path::PathBuf;

type LdexpQ2Fn = unsafe extern "C" fn(f32, i32) -> f32;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_c_lib() -> Library {
    let path = project_root().join("c_src/build/libtranslated_rust.so");
    unsafe { Library::new(&path).expect("failed to load C .so") }
}

fn load_rust_lib() -> Library {
    let path = project_root().join("target/debug/libldexp_q2_lib.so");
    unsafe { Library::new(&path).expect("failed to load Rust .so") }
}

#[test]
fn test_ldexp_q2_matches() {
    let c_lib = load_c_lib();
    let rs_lib = load_rust_lib();
    let c_fn: Symbol<LdexpQ2Fn> = unsafe { c_lib.get(b"ldexp_q2").unwrap() };
    let rs_fn: Symbol<LdexpQ2Fn> = unsafe { rs_lib.get(b"ldexp_q2").unwrap() };

    // Test inputs: (y, exp_q2) — only positive exp_q2 exercises the loop
    let cases: Vec<(f32, i32)> = vec![
        // Basic cases
        (1.0, 1),
        (1.0, 2),
        (1.0, 3),
        (1.0, 4),
        (1.0, 8),
        (1.0, 16),
        (1.0, 30 * 4),
        // Multiple loop iterations
        (1.0, 30 * 4 + 1),
        (1.0, 30 * 4 + 2),
        (1.0, 30 * 4 + 3),
        (1.0, 30 * 8),
        // Various y values
        (0.0, 4),
        (-1.0, 4),
        (0.5, 4),
        (100.0, 1),
        (1e10, 8),
        (1e-10, 8),
        (f32::MAX, 1),
        (f32::MIN_POSITIVE, 1),
        (-0.0, 4),
        (f32::INFINITY, 4),
        (f32::NEG_INFINITY, 4),
        (f32::NAN, 4),
        // Edge exp_q2 values
        (1.0, 119),
        (1.0, 120),
        // Sweep exp_q2 1..=120
    ];

    for &(y, exp_q2) in &cases {
        let c_result = unsafe { c_fn(y, exp_q2) };
        let rs_result = unsafe { rs_fn(y, exp_q2) };
        assert!(
            c_result.to_bits() == rs_result.to_bits(),
            "MISMATCH ldexp_q2({y}, {exp_q2}): C={c_result} (0x{:08x}), Rust={rs_result} (0x{:08x})",
            c_result.to_bits(),
            rs_result.to_bits(),
        );
    }

    // Sweep all exp_q2 from 1..=120 with a few y values
    for &y in &[1.0_f32, -3.5, 1e-20, 1e20] {
        for exp_q2 in 1..=120 {
            let c_result = unsafe { c_fn(y, exp_q2) };
            let rs_result = unsafe { rs_fn(y, exp_q2) };
            assert!(
                c_result.to_bits() == rs_result.to_bits(),
                "MISMATCH ldexp_q2({y}, {exp_q2}): C={c_result} (0x{:08x}), Rust={rs_result} (0x{:08x})",
                c_result.to_bits(),
                rs_result.to_bits(),
            );
        }
    }
}
