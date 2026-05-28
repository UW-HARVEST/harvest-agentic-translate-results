use libloading::{Library, Symbol};
use std::os::raw::c_int;

type LdexpQ2Fn = unsafe extern "C" fn(f32, c_int) -> f32;

const C_LIB_PATH: &str = "c_src/build/libtranslated_rust.so";
const RUST_LIB_PATH: &str = "target/release/libldexp_q2_lib.so";

fn call_lib(lib_path: &str, y: f32, exp_q2: c_int) -> f32 {
    unsafe {
        let lib = Library::new(lib_path).expect("failed to load lib");
        let func: Symbol<LdexpQ2Fn> = lib.get(b"ldexp_q2").expect("missing symbol");
        func(y, exp_q2)
    }
}

fn assert_match(y: f32, exp_q2: c_int) {
    let c_result = call_lib(C_LIB_PATH, y, exp_q2);
    let r_result = call_lib(RUST_LIB_PATH, y, exp_q2);
    assert_eq!(
        c_result.to_bits(),
        r_result.to_bits(),
        "mismatch for y={}, exp_q2={}: C=0x{:08x} ({}), Rust=0x{:08x} ({})",
        y,
        exp_q2,
        c_result.to_bits(),
        c_result,
        r_result.to_bits(),
        r_result,
    );
}

#[test]
fn test_basic_positive() {
    // exp_q2 in [1, 200], y = 1.0
    for exp_q2 in 1..=200 {
        assert_match(1.0, exp_q2);
    }
}

#[test]
fn test_zero_exp() {
    // exp_q2 == 0: e = 0, factor = (1<<30) >> 0 = (1<<30), idx=0
    assert_match(1.0, 0);
    assert_match(2.0, 0);
    assert_match(0.0, 0);
}

#[test]
fn test_various_y() {
    let ys = [
        0.0f32, -0.0f32, 1.0, -1.0, 2.0, -2.0, 0.5, 3.14159, 1e-10, 1e10,
    ];
    for &y in &ys {
        for exp_q2 in [0, 1, 2, 3, 4, 5, 10, 50, 100, 120, 121, 200, 240, 480] {
            assert_match(y, exp_q2);
        }
    }
}

#[test]
fn test_boundary_exp() {
    // e is clamped to 120
    for exp_q2 in [119, 120, 121, 122, 239, 240, 241, 360, 480, 500] {
        assert_match(1.0, exp_q2);
        assert_match(2.5, exp_q2);
    }
}

#[test]
fn test_special_y() {
    // NaN, infinities (multiplications by finite values)
    assert_match(f32::INFINITY, 1);
    assert_match(f32::NEG_INFINITY, 1);
    assert_match(f32::INFINITY, 0);
    assert_match(f32::NEG_INFINITY, 0);
}
