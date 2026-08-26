use libloading::{Library, Symbol};
use std::os::raw::c_int;

fn load_libs() -> (Library, Library) {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let c_path = format!("{}/c_src/build/libtranslated_rust.so", manifest);
    let rust_path = format!("{}/target/debug/libpow43_lib.so", manifest);
    unsafe {
        (
            Library::new(&c_path).expect("failed to load C .so"),
            Library::new(&rust_path).expect("failed to load Rust .so"),
        )
    }
}

#[test]
fn test_pow43_all_table_values() {
    let (c_lib, rs_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> f32> = c_lib.get(b"pow43").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(c_int) -> f32> = rs_lib.get(b"pow43").unwrap();

        // Test all values that hit the table lookup (x < 129)
        for x in 0..129 {
            let c_val = c_fn(x);
            let rs_val = rs_fn(x);
            assert_eq!(
                c_val.to_bits(),
                rs_val.to_bits(),
                "pow43({x}): C={c_val} Rust={rs_val}"
            );
        }
    }
}

#[test]
fn test_pow43_mid_range() {
    let (c_lib, rs_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> f32> = c_lib.get(b"pow43").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(c_int) -> f32> = rs_lib.get(b"pow43").unwrap();

        // Test range 129..1024 (mult=16, x<<=3 path)
        for x in 129..1024 {
            let c_val = c_fn(x);
            let rs_val = rs_fn(x);
            assert_eq!(
                c_val.to_bits(),
                rs_val.to_bits(),
                "pow43({x}): C={c_val} Rust={rs_val}"
            );
        }
    }
}

#[test]
fn test_pow43_high_range() {
    let (c_lib, rs_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> f32> = c_lib.get(b"pow43").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(c_int) -> f32> = rs_lib.get(b"pow43").unwrap();

        // Test range 1024..8207 (mult=256 path)
        // 8207 = 16 + (129-1)*64 + 63 is the max safe index into g_pow43
        for x in 1024..8207 {
            let c_val = c_fn(x);
            let rs_val = rs_fn(x);
            assert_eq!(
                c_val.to_bits(),
                rs_val.to_bits(),
                "pow43({x}): C={c_val} Rust={rs_val}"
            );
        }
    }
}

#[test]
fn test_pow43_negative_inputs() {
    let (c_lib, rs_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> f32> = c_lib.get(b"pow43").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(c_int) -> f32> = rs_lib.get(b"pow43").unwrap();

        // Negative values always hit the table path (x < 129)
        // Valid negative indices: 16 + x >= 0, so x >= -16
        for x in -16..0 {
            let c_val = c_fn(x);
            let rs_val = rs_fn(x);
            assert_eq!(
                c_val.to_bits(),
                rs_val.to_bits(),
                "pow43({x}): C={c_val} Rust={rs_val}"
            );
        }
    }
}
