use libloading::{Library, Symbol};

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libstb_perlin.so");

fn load_c_lib() -> Library {
    unsafe { Library::new(C_LIB_PATH).expect("Failed to load C shared library") }
}

// Test inputs: various (x,y,z) values with different wrap/seed combos
struct NoiseInput {
    x: f32, y: f32, z: f32,
    x_wrap: i32, y_wrap: i32, z_wrap: i32,
    seed: i32,
}

fn test_inputs() -> Vec<NoiseInput> {
    vec![
        NoiseInput { x: 0.0, y: 0.0, z: 0.0, x_wrap: 0, y_wrap: 0, z_wrap: 0, seed: 0 },
        NoiseInput { x: 1.5, y: 2.3, z: 0.7, x_wrap: 0, y_wrap: 0, z_wrap: 0, seed: 0 },
        NoiseInput { x: -3.2, y: 4.1, z: -1.8, x_wrap: 0, y_wrap: 0, z_wrap: 0, seed: 0 },
        NoiseInput { x: 0.5, y: 0.5, z: 0.5, x_wrap: 4, y_wrap: 4, z_wrap: 4, seed: 0 },
        NoiseInput { x: 10.0, y: 20.0, z: 30.0, x_wrap: 8, y_wrap: 8, z_wrap: 8, seed: 42 },
        NoiseInput { x: 0.123, y: 0.456, z: 0.789, x_wrap: 0, y_wrap: 0, z_wrap: 0, seed: 255 },
        NoiseInput { x: 100.0, y: -50.0, z: 25.0, x_wrap: 16, y_wrap: 16, z_wrap: 16, seed: 7 },
        NoiseInput { x: 0.001, y: 0.001, z: 0.001, x_wrap: 0, y_wrap: 0, z_wrap: 0, seed: 0 },
        NoiseInput { x: 255.0, y: 255.0, z: 255.0, x_wrap: 0, y_wrap: 0, z_wrap: 0, seed: 128 },
        NoiseInput { x: -0.5, y: -0.5, z: -0.5, x_wrap: 0, y_wrap: 0, z_wrap: 0, seed: 1 },
    ]
}

#[test]
fn test_noise3_internal() {
    let lib = load_c_lib();
    type Fn = unsafe extern "C" fn(f32, f32, f32, i32, i32, i32, u8) -> f32;
    let c_fn: Symbol<Fn> = unsafe { lib.get(b"stb_perlin_noise3_internal").unwrap() };

    for inp in test_inputs() {
        let seed = inp.seed as u8;
        let c_result = unsafe { c_fn(inp.x, inp.y, inp.z, inp.x_wrap, inp.y_wrap, inp.z_wrap, seed) };
        let rust_result = driver::stb_perlin_noise3_internal(inp.x, inp.y, inp.z, inp.x_wrap, inp.y_wrap, inp.z_wrap, seed);
        assert_eq!(
            c_result.to_bits(), rust_result.to_bits(),
            "noise3_internal mismatch for ({},{},{}) wrap=({},{},{}) seed={}: C={} Rust={}",
            inp.x, inp.y, inp.z, inp.x_wrap, inp.y_wrap, inp.z_wrap, seed, c_result, rust_result
        );
    }
}

#[test]
fn test_noise3() {
    let lib = load_c_lib();
    type Fn = unsafe extern "C" fn(f32, f32, f32, i32, i32, i32) -> f32;
    let c_fn: Symbol<Fn> = unsafe { lib.get(b"stb_perlin_noise3").unwrap() };

    for inp in test_inputs() {
        let c_result = unsafe { c_fn(inp.x, inp.y, inp.z, inp.x_wrap, inp.y_wrap, inp.z_wrap) };
        let rust_result = driver::stb_perlin_noise3(inp.x, inp.y, inp.z, inp.x_wrap, inp.y_wrap, inp.z_wrap);
        assert_eq!(
            c_result.to_bits(), rust_result.to_bits(),
            "noise3 mismatch for ({},{},{}) wrap=({},{},{}): C={} Rust={}",
            inp.x, inp.y, inp.z, inp.x_wrap, inp.y_wrap, inp.z_wrap, c_result, rust_result
        );
    }
}

#[test]
fn test_noise3_seed() {
    let lib = load_c_lib();
    type Fn = unsafe extern "C" fn(f32, f32, f32, i32, i32, i32, i32) -> f32;
    let c_fn: Symbol<Fn> = unsafe { lib.get(b"stb_perlin_noise3_seed").unwrap() };

    for inp in test_inputs() {
        let c_result = unsafe { c_fn(inp.x, inp.y, inp.z, inp.x_wrap, inp.y_wrap, inp.z_wrap, inp.seed) };
        let rust_result = driver::stb_perlin_noise3_seed(inp.x, inp.y, inp.z, inp.x_wrap, inp.y_wrap, inp.z_wrap, inp.seed);
        assert_eq!(
            c_result.to_bits(), rust_result.to_bits(),
            "noise3_seed mismatch for ({},{},{}) wrap=({},{},{}) seed={}: C={} Rust={}",
            inp.x, inp.y, inp.z, inp.x_wrap, inp.y_wrap, inp.z_wrap, inp.seed, c_result, rust_result
        );
    }
}

struct FractalInput {
    x: f32, y: f32, z: f32,
    lacunarity: f32, gain: f32, offset: f32, octaves: i32,
}

fn fractal_inputs() -> Vec<FractalInput> {
    vec![
        FractalInput { x: 0.5, y: 0.5, z: 0.5, lacunarity: 2.0, gain: 0.5, offset: 1.0, octaves: 6 },
        FractalInput { x: 1.0, y: 2.0, z: 3.0, lacunarity: 2.0, gain: 0.5, offset: 1.0, octaves: 4 },
        FractalInput { x: -1.5, y: 0.7, z: 2.3, lacunarity: 1.5, gain: 0.6, offset: 0.8, octaves: 3 },
        FractalInput { x: 0.0, y: 0.0, z: 0.0, lacunarity: 2.0, gain: 0.5, offset: 1.0, octaves: 1 },
        FractalInput { x: 10.5, y: -3.2, z: 7.1, lacunarity: 2.5, gain: 0.4, offset: 1.2, octaves: 8 },
    ]
}

#[test]
fn test_ridge_noise3() {
    let lib = load_c_lib();
    type Fn = unsafe extern "C" fn(f32, f32, f32, f32, f32, f32, i32) -> f32;
    let c_fn: Symbol<Fn> = unsafe { lib.get(b"stb_perlin_ridge_noise3").unwrap() };

    for inp in fractal_inputs() {
        let c_result = unsafe { c_fn(inp.x, inp.y, inp.z, inp.lacunarity, inp.gain, inp.offset, inp.octaves) };
        let rust_result = driver::stb_perlin_ridge_noise3(inp.x, inp.y, inp.z, inp.lacunarity, inp.gain, inp.offset, inp.octaves);
        assert_eq!(
            c_result.to_bits(), rust_result.to_bits(),
            "ridge_noise3 mismatch for ({},{},{}) lac={} gain={} off={} oct={}: C={} Rust={}",
            inp.x, inp.y, inp.z, inp.lacunarity, inp.gain, inp.offset, inp.octaves, c_result, rust_result
        );
    }
}

#[test]
fn test_fbm_noise3() {
    let lib = load_c_lib();
    type Fn = unsafe extern "C" fn(f32, f32, f32, f32, f32, i32) -> f32;
    let c_fn: Symbol<Fn> = unsafe { lib.get(b"stb_perlin_fbm_noise3").unwrap() };

    for inp in fractal_inputs() {
        let c_result = unsafe { c_fn(inp.x, inp.y, inp.z, inp.lacunarity, inp.gain, inp.octaves) };
        let rust_result = driver::stb_perlin_fbm_noise3(inp.x, inp.y, inp.z, inp.lacunarity, inp.gain, inp.octaves);
        assert_eq!(
            c_result.to_bits(), rust_result.to_bits(),
            "fbm_noise3 mismatch for ({},{},{}) lac={} gain={} oct={}: C={} Rust={}",
            inp.x, inp.y, inp.z, inp.lacunarity, inp.gain, inp.octaves, c_result, rust_result
        );
    }
}

#[test]
fn test_turbulence_noise3() {
    let lib = load_c_lib();
    type Fn = unsafe extern "C" fn(f32, f32, f32, f32, f32, i32) -> f32;
    let c_fn: Symbol<Fn> = unsafe { lib.get(b"stb_perlin_turbulence_noise3").unwrap() };

    for inp in fractal_inputs() {
        let c_result = unsafe { c_fn(inp.x, inp.y, inp.z, inp.lacunarity, inp.gain, inp.octaves) };
        let rust_result = driver::stb_perlin_turbulence_noise3(inp.x, inp.y, inp.z, inp.lacunarity, inp.gain, inp.octaves);
        assert_eq!(
            c_result.to_bits(), rust_result.to_bits(),
            "turbulence_noise3 mismatch for ({},{},{}) lac={} gain={} oct={}: C={} Rust={}",
            inp.x, inp.y, inp.z, inp.lacunarity, inp.gain, inp.octaves, c_result, rust_result
        );
    }
}

#[test]
fn test_noise3_wrap_nonpow2() {
    let lib = load_c_lib();
    type Fn = unsafe extern "C" fn(f32, f32, f32, i32, i32, i32, u8) -> f32;
    let c_fn: Symbol<Fn> = unsafe { lib.get(b"stb_perlin_noise3_wrap_nonpow2").unwrap() };

    let inputs = vec![
        (0.5f32, 0.5f32, 0.5f32, 0, 0, 0, 0u8),
        (1.5, 2.3, 0.7, 3, 5, 7, 0),
        (-3.2, 4.1, -1.8, 10, 10, 10, 42),
        (0.5, 0.5, 0.5, 4, 4, 4, 0),
        (10.0, 20.0, 30.0, 6, 9, 12, 100),
        (0.123, 0.456, 0.789, 0, 0, 0, 255),
        (-0.5, -0.5, -0.5, 7, 7, 7, 1),
    ];

    for (x, y, z, xw, yw, zw, seed) in inputs {
        let c_result = unsafe { c_fn(x, y, z, xw, yw, zw, seed) };
        let rust_result = driver::stb_perlin_noise3_wrap_nonpow2(x, y, z, xw, yw, zw, seed);
        assert_eq!(
            c_result.to_bits(), rust_result.to_bits(),
            "noise3_wrap_nonpow2 mismatch for ({},{},{}) wrap=({},{},{}) seed={}: C={} Rust={}",
            x, y, z, xw, yw, zw, seed, c_result, rust_result
        );
    }
}
