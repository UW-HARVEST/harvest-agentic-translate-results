use libloading::Library;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libstb_perlin.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libstb_perlin.so")
}

type Fn3Wrap = unsafe extern "C" fn(f32, f32, f32, i32, i32, i32) -> f32;
type Fn3Internal = unsafe extern "C" fn(f32, f32, f32, i32, i32, i32, u8) -> f32;
type Fn3Seed = unsafe extern "C" fn(f32, f32, f32, i32, i32, i32, i32) -> f32;
type FnRidge = unsafe extern "C" fn(f32, f32, f32, f32, f32, f32, i32) -> f32;
type FnFbm = unsafe extern "C" fn(f32, f32, f32, f32, f32, i32) -> f32;
type FnWrapNp = unsafe extern "C" fn(f32, f32, f32, i32, i32, i32, u8) -> f32;

fn assert_bits_eq(c_val: f32, rs_val: f32, ctx: &str) {
    assert_eq!(
        c_val.to_bits(), rs_val.to_bits(),
        "{}: C={} (0x{:08x}) != Rust={} (0x{:08x})",
        ctx, c_val, c_val.to_bits(), rs_val, rs_val.to_bits()
    );
}

const COORDS: &[(f32, f32, f32)] = &[
    (0.0, 0.0, 0.0),
    (0.5, 0.5, 0.5),
    (1.0, 2.0, 3.0),
    (-0.5, -0.5, -0.5),
    (3.14, 2.71, 1.41),
    (100.0, 200.0, 300.0),
    (0.001, 0.002, 0.003),
    (-10.5, 20.3, -30.7),
    (255.0, 255.0, 255.0),
    (0.123, 4.567, 8.901),
];

#[test]
fn test_noise3_internal() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_fn = unsafe { c_lib.get::<Fn3Internal>(b"stb_perlin_noise3_internal").unwrap() };
    let rs_fn = unsafe { rs_lib.get::<Fn3Internal>(b"stb_perlin_noise3_internal").unwrap() };
    for &(x, y, z) in COORDS {
        for seed in [0u8, 1, 42, 127, 255] {
            for &(xw, yw, zw) in &[(0,0,0), (4,4,4), (8,16,32), (256,256,256)] {
                let c = unsafe { c_fn(x, y, z, xw, yw, zw, seed) };
                let r = unsafe { rs_fn(x, y, z, xw, yw, zw, seed) };
                assert_bits_eq(c, r, &format!("internal({x},{y},{z},{xw},{yw},{zw},{seed})"));
            }
        }
    }
}

#[test]
fn test_noise3() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_fn = unsafe { c_lib.get::<Fn3Wrap>(b"stb_perlin_noise3").unwrap() };
    let rs_fn = unsafe { rs_lib.get::<Fn3Wrap>(b"stb_perlin_noise3").unwrap() };
    for &(x, y, z) in COORDS {
        for &(xw, yw, zw) in &[(0,0,0), (4,4,4), (16,16,16)] {
            let c = unsafe { c_fn(x, y, z, xw, yw, zw) };
            let r = unsafe { rs_fn(x, y, z, xw, yw, zw) };
            assert_bits_eq(c, r, &format!("noise3({x},{y},{z},{xw},{yw},{zw})"));
        }
    }
}

#[test]
fn test_noise3_seed() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_fn = unsafe { c_lib.get::<Fn3Seed>(b"stb_perlin_noise3_seed").unwrap() };
    let rs_fn = unsafe { rs_lib.get::<Fn3Seed>(b"stb_perlin_noise3_seed").unwrap() };
    for &(x, y, z) in COORDS {
        for seed in [0i32, 1, 42, 127, 255, 256, -1] {
            for &(xw, yw, zw) in &[(0,0,0), (8,8,8)] {
                let c = unsafe { c_fn(x, y, z, xw, yw, zw, seed) };
                let r = unsafe { rs_fn(x, y, z, xw, yw, zw, seed) };
                assert_bits_eq(c, r, &format!("seed({x},{y},{z},{xw},{yw},{zw},{seed})"));
            }
        }
    }
}

#[test]
fn test_ridge_noise3() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_fn = unsafe { c_lib.get::<FnRidge>(b"stb_perlin_ridge_noise3").unwrap() };
    let rs_fn = unsafe { rs_lib.get::<FnRidge>(b"stb_perlin_ridge_noise3").unwrap() };
    for &(x, y, z) in COORDS {
        for &(lac, gain, off, oct) in &[
            (2.0f32, 0.5f32, 1.0f32, 6i32),
            (2.0, 0.5, 1.0, 1),
            (1.5, 0.7, 0.5, 4),
            (2.0, 0.5, 1.0, 0),
        ] {
            let c = unsafe { c_fn(x, y, z, lac, gain, off, oct) };
            let r = unsafe { rs_fn(x, y, z, lac, gain, off, oct) };
            assert_bits_eq(c, r, &format!("ridge({x},{y},{z},{lac},{gain},{off},{oct})"));
        }
    }
}

#[test]
fn test_fbm_noise3() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_fn = unsafe { c_lib.get::<FnFbm>(b"stb_perlin_fbm_noise3").unwrap() };
    let rs_fn = unsafe { rs_lib.get::<FnFbm>(b"stb_perlin_fbm_noise3").unwrap() };
    for &(x, y, z) in COORDS {
        for &(lac, gain, oct) in &[
            (2.0f32, 0.5f32, 6i32),
            (2.0, 0.5, 1),
            (1.5, 0.7, 4),
            (2.0, 0.5, 0),
        ] {
            let c = unsafe { c_fn(x, y, z, lac, gain, oct) };
            let r = unsafe { rs_fn(x, y, z, lac, gain, oct) };
            assert_bits_eq(c, r, &format!("fbm({x},{y},{z},{lac},{gain},{oct})"));
        }
    }
}

#[test]
fn test_turbulence_noise3() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_fn = unsafe { c_lib.get::<FnFbm>(b"stb_perlin_turbulence_noise3").unwrap() };
    let rs_fn = unsafe { rs_lib.get::<FnFbm>(b"stb_perlin_turbulence_noise3").unwrap() };
    for &(x, y, z) in COORDS {
        for &(lac, gain, oct) in &[
            (2.0f32, 0.5f32, 6i32),
            (2.0, 0.5, 1),
            (1.5, 0.7, 4),
            (2.0, 0.5, 0),
        ] {
            let c = unsafe { c_fn(x, y, z, lac, gain, oct) };
            let r = unsafe { rs_fn(x, y, z, lac, gain, oct) };
            assert_bits_eq(c, r, &format!("turb({x},{y},{z},{lac},{gain},{oct})"));
        }
    }
}

#[test]
fn test_noise3_wrap_nonpow2() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };
    let c_fn = unsafe { c_lib.get::<FnWrapNp>(b"stb_perlin_noise3_wrap_nonpow2").unwrap() };
    let rs_fn = unsafe { rs_lib.get::<FnWrapNp>(b"stb_perlin_noise3_wrap_nonpow2").unwrap() };
    for &(x, y, z) in COORDS {
        for seed in [0u8, 1, 42, 255] {
            for &(xw, yw, zw) in &[(0,0,0), (3,5,7), (10,10,10), (256,256,256), (100,200,50)] {
                let c = unsafe { c_fn(x, y, z, xw, yw, zw, seed) };
                let r = unsafe { rs_fn(x, y, z, xw, yw, zw, seed) };
                assert_bits_eq(c, r, &format!("nonpow2({x},{y},{z},{xw},{yw},{zw},{seed})"));
            }
        }
    }
}
