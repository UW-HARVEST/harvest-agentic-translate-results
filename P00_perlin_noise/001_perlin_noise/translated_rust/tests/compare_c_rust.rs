use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libperlin.so")
}

macro_rules! load_fn {
    ($lib:expr, $name:expr, $ty:ty) => {
        unsafe { $lib.get::<$ty>($name).expect(concat!("Failed to load ", stringify!($name))) }
    };
}

/// Test inputs covering various cases: zero, fractional, negative, wrapping
const TEST_COORDS: [(f32, f32, f32); 8] = [
    (0.0, 0.0, 0.0),
    (0.5, 0.5, 0.5),
    (1.0, 2.0, 3.0),
    (-0.7, 1.3, -2.1),
    (0.123, 0.456, 0.789),
    (10.5, -3.2, 7.8),
    (255.0, 255.0, 255.0),
    (0.001, 0.002, 0.003),
];

#[test]
fn test_noise3_internal() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };
    type Fn = unsafe extern "C" fn(f32, f32, f32, i32, i32, i32, u8) -> f32;
    let c_fn = load_fn!(lib, b"stb_perlin_noise3_internal\0", Fn);

    for &(x, y, z) in &TEST_COORDS {
        for seed in [0u8, 1, 42, 255] {
            for &(xw, yw, zw) in &[(0, 0, 0), (0, 0, 0), (16, 16, 16)] {
                let c_val = unsafe { c_fn(x, y, z, xw, yw, zw, seed) };
                let r_val = perlin_noise::stb_perlin_noise3_internal(x, y, z, xw, yw, zw, seed);
                assert_eq!(
                    c_val.to_bits(), r_val.to_bits(),
                    "noise3_internal mismatch at ({x},{y},{z}) wrap=({xw},{yw},{zw}) seed={seed}: C={c_val} Rust={r_val}"
                );
            }
        }
    }
}

#[test]
fn test_noise3() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };
    type Fn = unsafe extern "C" fn(f32, f32, f32, i32, i32, i32) -> f32;
    let c_fn = load_fn!(lib, b"stb_perlin_noise3\0", Fn);

    for &(x, y, z) in &TEST_COORDS {
        for &(xw, yw, zw) in &[(0, 0, 0), (16, 16, 16), (8, 4, 2)] {
            let c_val = unsafe { c_fn(x, y, z, xw, yw, zw) };
            let r_val = perlin_noise::stb_perlin_noise3(x, y, z, xw, yw, zw);
            assert_eq!(
                c_val.to_bits(), r_val.to_bits(),
                "noise3 mismatch at ({x},{y},{z}) wrap=({xw},{yw},{zw}): C={c_val} Rust={r_val}"
            );
        }
    }
}

#[test]
fn test_noise3_seed() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };
    type Fn = unsafe extern "C" fn(f32, f32, f32, i32, i32, i32, i32) -> f32;
    let c_fn = load_fn!(lib, b"stb_perlin_noise3_seed\0", Fn);

    for &(x, y, z) in &TEST_COORDS {
        for seed in [0i32, 1, 42, 255, 256] {
            let c_val = unsafe { c_fn(x, y, z, 0, 0, 0, seed) };
            let r_val = perlin_noise::stb_perlin_noise3_seed(x, y, z, 0, 0, 0, seed);
            assert_eq!(
                c_val.to_bits(), r_val.to_bits(),
                "noise3_seed mismatch at ({x},{y},{z}) seed={seed}: C={c_val} Rust={r_val}"
            );
        }
    }
}

#[test]
fn test_ridge_noise3() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };
    type Fn = unsafe extern "C" fn(f32, f32, f32, f32, f32, f32, i32) -> f32;
    let c_fn = load_fn!(lib, b"stb_perlin_ridge_noise3\0", Fn);

    for &(x, y, z) in &TEST_COORDS {
        for &(lac, gain, off, oct) in &[(2.0f32, 0.5f32, 1.0f32, 6i32), (1.5, 0.6, 1.2, 4), (2.0, 0.5, 1.0, 1)] {
            let c_val = unsafe { c_fn(x, y, z, lac, gain, off, oct) };
            let r_val = perlin_noise::stb_perlin_ridge_noise3(x, y, z, lac, gain, off, oct);
            assert_eq!(
                c_val.to_bits(), r_val.to_bits(),
                "ridge_noise3 mismatch at ({x},{y},{z}) lac={lac} gain={gain} off={off} oct={oct}: C={c_val} Rust={r_val}"
            );
        }
    }
}

#[test]
fn test_fbm_noise3() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };
    type Fn = unsafe extern "C" fn(f32, f32, f32, f32, f32, i32) -> f32;
    let c_fn = load_fn!(lib, b"stb_perlin_fbm_noise3\0", Fn);

    for &(x, y, z) in &TEST_COORDS {
        for &(lac, gain, oct) in &[(2.0f32, 0.5f32, 6i32), (1.5, 0.6, 4), (2.0, 0.5, 1)] {
            let c_val = unsafe { c_fn(x, y, z, lac, gain, oct) };
            let r_val = perlin_noise::stb_perlin_fbm_noise3(x, y, z, lac, gain, oct);
            assert_eq!(
                c_val.to_bits(), r_val.to_bits(),
                "fbm_noise3 mismatch at ({x},{y},{z}) lac={lac} gain={gain} oct={oct}: C={c_val} Rust={r_val}"
            );
        }
    }
}

#[test]
fn test_turbulence_noise3() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };
    type Fn = unsafe extern "C" fn(f32, f32, f32, f32, f32, i32) -> f32;
    let c_fn = load_fn!(lib, b"stb_perlin_turbulence_noise3\0", Fn);

    for &(x, y, z) in &TEST_COORDS {
        for &(lac, gain, oct) in &[(2.0f32, 0.5f32, 6i32), (1.5, 0.6, 4), (2.0, 0.5, 1)] {
            let c_val = unsafe { c_fn(x, y, z, lac, gain, oct) };
            let r_val = perlin_noise::stb_perlin_turbulence_noise3(x, y, z, lac, gain, oct);
            assert_eq!(
                c_val.to_bits(), r_val.to_bits(),
                "turbulence_noise3 mismatch at ({x},{y},{z}) lac={lac} gain={gain} oct={oct}: C={c_val} Rust={r_val}"
            );
        }
    }
}

#[test]
fn test_noise3_wrap_nonpow2() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };
    type Fn = unsafe extern "C" fn(f32, f32, f32, i32, i32, i32, u8) -> f32;
    let c_fn = load_fn!(lib, b"stb_perlin_noise3_wrap_nonpow2\0", Fn);

    for &(x, y, z) in &TEST_COORDS {
        for seed in [0u8, 1, 42] {
            for &(xw, yw, zw) in &[(0, 0, 0), (10, 10, 10), (7, 13, 5)] {
                let c_val = unsafe { c_fn(x, y, z, xw, yw, zw, seed) };
                let r_val = perlin_noise::stb_perlin_noise3_wrap_nonpow2(x, y, z, xw, yw, zw, seed);
                assert_eq!(
                    c_val.to_bits(), r_val.to_bits(),
                    "noise3_wrap_nonpow2 mismatch at ({x},{y},{z}) wrap=({xw},{yw},{zw}) seed={seed}: C={c_val} Rust={r_val}"
                );
            }
        }
    }
}

#[test]
fn test_inner() {
    let lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };
    type Fn = unsafe extern "C" fn(i32, f32, f32, f32, i32, i32, i32, i32, f32, f32, f32, i32) -> f32;
    let c_fn = load_fn!(lib, b"inner\0", Fn);

    // Test each which value with representative inputs
    let cases: Vec<(i32, f32, f32, f32, i32, i32, i32, i32, f32, f32, f32, i32)> = vec![
        (0, 0.5, 0.5, 0.5, 0, 0, 0, 0, 0.0, 0.0, 0.0, 0),
        (1, 0.5, 0.5, 0.5, 0, 0, 0, 42, 0.0, 0.0, 0.0, 0),
        (2, 0.5, 0.5, 0.5, 0, 0, 0, 0, 2.0, 0.5, 1.0, 6),
        (3, 0.5, 0.5, 0.5, 0, 0, 0, 0, 2.0, 0.5, 0.0, 6),
        (4, 0.5, 0.5, 0.5, 0, 0, 0, 0, 2.0, 0.5, 0.0, 6),
        (5, 0.5, 0.5, 0.5, 10, 10, 10, 1, 0.0, 0.0, 0.0, 0),
        (99, 0.0, 0.0, 0.0, 0, 0, 0, 0, 0.0, 0.0, 0.0, 0), // default -> NaN
    ];

    for &(which, x, y, z, xw, yw, zw, seed, lac, gain, off, oct) in &cases {
        let c_val = unsafe { c_fn(which, x, y, z, xw, yw, zw, seed, lac, gain, off, oct) };
        let r_val = perlin_noise::inner(which, x, y, z, xw, yw, zw, seed, lac, gain, off, oct);
        assert_eq!(
            c_val.to_bits(), r_val.to_bits(),
            "inner mismatch which={which}: C={c_val} Rust={r_val}"
        );
    }
}
