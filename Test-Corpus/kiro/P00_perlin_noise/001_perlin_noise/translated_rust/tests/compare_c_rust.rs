use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("c_src/build/libperlin.so")
}

macro_rules! assert_bits_eq {
    ($c:expr, $r:expr, $label:expr) => {
        assert!(
            $c.to_bits() == $r.to_bits(),
            "{}: C={} (0x{:08x}) != Rust={} (0x{:08x})",
            $label, $c, $c.to_bits(), $r, $r.to_bits()
        );
    };
}

// Test inputs: a variety of coordinates covering typical usage
static TEST_COORDS: [(f32, f32, f32); 8] = [
    (0.0, 0.0, 0.0),
    (1.5, 2.3, 0.7),
    (-0.5, 1.0, -1.0),
    (3.14, 2.71, 1.41),
    (100.0, 200.0, 300.0),
    (0.001, 0.002, 0.003),
    (-10.5, 5.5, -3.3),
    (0.5, 0.5, 0.5),
];

#[test]
fn test_noise3_internal() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let c_fn: Symbol<unsafe extern "C" fn(f32, f32, f32, i32, i32, i32, u8) -> f32> =
            lib.get(b"stb_perlin_noise3_internal").unwrap();

        for &(x, y, z) in &TEST_COORDS {
            for seed in [0u8, 1, 42, 255] {
                let c_val = c_fn(x, y, z, 0, 0, 0, seed);
                let r_val = perlin_noise::stb_perlin_noise3_internal(x, y, z, 0, 0, 0, seed);
                assert_bits_eq!(c_val, r_val, format!("noise3_internal({x},{y},{z},0,0,0,{seed})"));
            }
        }
        // With wrapping
        for &(x, y, z) in &TEST_COORDS[..4] {
            for wrap in [4, 8, 16, 256] {
                let c_val = c_fn(x, y, z, wrap, wrap, wrap, 0);
                let r_val = perlin_noise::stb_perlin_noise3_internal(x, y, z, wrap, wrap, wrap, 0);
                assert_bits_eq!(c_val, r_val, format!("noise3_internal({x},{y},{z},{wrap},{wrap},{wrap},0)"));
            }
        }
    }
}

#[test]
fn test_noise3() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let c_fn: Symbol<unsafe extern "C" fn(f32, f32, f32, i32, i32, i32) -> f32> =
            lib.get(b"stb_perlin_noise3").unwrap();

        for &(x, y, z) in &TEST_COORDS {
            let c_val = c_fn(x, y, z, 0, 0, 0);
            let r_val = perlin_noise::stb_perlin_noise3(x, y, z, 0, 0, 0);
            assert_bits_eq!(c_val, r_val, format!("noise3({x},{y},{z},0,0,0)"));
        }
    }
}

#[test]
fn test_noise3_seed() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let c_fn: Symbol<unsafe extern "C" fn(f32, f32, f32, i32, i32, i32, i32) -> f32> =
            lib.get(b"stb_perlin_noise3_seed").unwrap();

        for &(x, y, z) in &TEST_COORDS {
            for seed in [0, 1, 42, 255, 256] {
                let c_val = c_fn(x, y, z, 0, 0, 0, seed);
                let r_val = perlin_noise::stb_perlin_noise3_seed(x, y, z, 0, 0, 0, seed);
                assert_bits_eq!(c_val, r_val, format!("noise3_seed({x},{y},{z},0,0,0,{seed})"));
            }
        }
    }
}

#[test]
fn test_fbm_noise3() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let c_fn: Symbol<unsafe extern "C" fn(f32, f32, f32, f32, f32, i32) -> f32> =
            lib.get(b"stb_perlin_fbm_noise3").unwrap();

        for &(x, y, z) in &TEST_COORDS {
            for octaves in [1, 3, 6] {
                let c_val = c_fn(x, y, z, 2.0, 0.5, octaves);
                let r_val = perlin_noise::stb_perlin_fbm_noise3(x, y, z, 2.0, 0.5, octaves);
                assert_bits_eq!(c_val, r_val, format!("fbm({x},{y},{z},2.0,0.5,{octaves})"));
            }
        }
    }
}

#[test]
fn test_ridge_noise3() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let c_fn: Symbol<unsafe extern "C" fn(f32, f32, f32, f32, f32, f32, i32) -> f32> =
            lib.get(b"stb_perlin_ridge_noise3").unwrap();

        for &(x, y, z) in &TEST_COORDS {
            for octaves in [1, 3, 6] {
                let c_val = c_fn(x, y, z, 2.0, 0.5, 1.0, octaves);
                let r_val = perlin_noise::stb_perlin_ridge_noise3(x, y, z, 2.0, 0.5, 1.0, octaves);
                assert_bits_eq!(c_val, r_val, format!("ridge({x},{y},{z},2.0,0.5,1.0,{octaves})"));
            }
        }
    }
}

#[test]
fn test_turbulence_noise3() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let c_fn: Symbol<unsafe extern "C" fn(f32, f32, f32, f32, f32, i32) -> f32> =
            lib.get(b"stb_perlin_turbulence_noise3").unwrap();

        for &(x, y, z) in &TEST_COORDS {
            for octaves in [1, 3, 6] {
                let c_val = c_fn(x, y, z, 2.0, 0.5, octaves);
                let r_val = perlin_noise::stb_perlin_turbulence_noise3(x, y, z, 2.0, 0.5, octaves);
                assert_bits_eq!(c_val, r_val, format!("turbulence({x},{y},{z},2.0,0.5,{octaves})"));
            }
        }
    }
}

#[test]
fn test_noise3_wrap_nonpow2() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let c_fn: Symbol<unsafe extern "C" fn(f32, f32, f32, i32, i32, i32, u8) -> f32> =
            lib.get(b"stb_perlin_noise3_wrap_nonpow2").unwrap();

        for &(x, y, z) in &TEST_COORDS {
            for wrap in [0, 3, 7, 10, 256] {
                for seed in [0u8, 1, 42] {
                    let c_val = c_fn(x, y, z, wrap, wrap, wrap, seed);
                    let r_val = perlin_noise::stb_perlin_noise3_wrap_nonpow2(x, y, z, wrap, wrap, wrap, seed);
                    assert_bits_eq!(c_val, r_val, format!("wrap_nonpow2({x},{y},{z},{wrap},{wrap},{wrap},{seed})"));
                }
            }
        }
    }
}

#[test]
fn test_inner() {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C library");
        let c_fn: Symbol<unsafe extern "C" fn(i32, f32, f32, f32, i32, i32, i32, i32, f32, f32, f32, i32) -> f32> =
            lib.get(b"inner").unwrap();

        let (x, y, z) = (1.5, 2.3, 0.7);
        for which in 0..=5 {
            let c_val = c_fn(which, x, y, z, 0, 0, 0, 42, 2.0, 0.5, 1.0, 6);
            let r_val = perlin_noise::inner(which, x, y, z, 0, 0, 0, 42, 2.0, 0.5, 1.0, 6);
            assert_bits_eq!(c_val, r_val, format!("inner({which},...)"));
        }
        // default case
        let c_val = c_fn(99, x, y, z, 0, 0, 0, 0, 0.0, 0.0, 0.0, 0);
        let r_val = perlin_noise::inner(99, x, y, z, 0, 0, 0, 0, 0.0, 0.0, 0.0, 0);
        assert!(c_val.is_nan() && r_val.is_nan(), "default case should be NaN");
    }
}
