// Integration test that loads the C and Rust shared libraries via libloading
// and compares their outputs byte-for-byte (bit-exact for f32 values).

use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_uchar};
use std::path::PathBuf;

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libcdriver.so");
    p
}

fn rust_so_path() -> PathBuf {
    // CARGO_TARGET_DIR? Use OUT_DIR-like discovery: the test binary lives in
    // target/<profile>/deps/<test-name>. We can read the libdriver.so from
    // the same target/<profile>/ directory. CARGO sets the env var
    // CARGO_TARGET_TMPDIR but not the lib path directly. Easiest:
    // current_exe().parent().parent().join("libdriver.so")
    let exe = std::env::current_exe().expect("current_exe");
    let mut p = exe.parent().unwrap().to_path_buf();
    if p.file_name().map(|n| n == "deps").unwrap_or(false) {
        p.pop();
    }
    p.push("libdriver.so");
    p
}

type InnerFn = unsafe extern "C" fn(
    c_int, f32, f32, f32, c_int, c_int, c_int, c_int, f32, f32, f32, c_int,
) -> f32;

type Noise3Fn = unsafe extern "C" fn(f32, f32, f32, c_int, c_int, c_int) -> f32;
type Noise3SeedFn =
    unsafe extern "C" fn(f32, f32, f32, c_int, c_int, c_int, c_int) -> f32;
type RidgeFn =
    unsafe extern "C" fn(f32, f32, f32, f32, f32, f32, c_int) -> f32;
type FbmFn = unsafe extern "C" fn(f32, f32, f32, f32, f32, c_int) -> f32;
type TurbFn = unsafe extern "C" fn(f32, f32, f32, f32, f32, c_int) -> f32;
type Wrap2Fn = unsafe extern "C" fn(
    f32, f32, f32, c_int, c_int, c_int, c_uchar,
) -> f32;
type InternalFn = unsafe extern "C" fn(
    f32, f32, f32, c_int, c_int, c_int, c_uchar,
) -> f32;

fn assert_f32_bit_eq(c: f32, r: f32, ctx: &str) {
    let cb = c.to_bits();
    let rb = r.to_bits();
    if cb != rb {
        // Both NaN with same payload? Allow if both are NaN at all.
        if c.is_nan() && r.is_nan() {
            return;
        }
        panic!(
            "Mismatch ({ctx}): C={c} (bits={cb:#010x}) vs Rust={r} (bits={rb:#010x})"
        );
    }
}

#[test]
fn test_inner_noise3() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let c_inner: Symbol<InnerFn> = c_lib.get(b"inner").unwrap();
        let r_inner: Symbol<InnerFn> = r_lib.get(b"inner").unwrap();

        let cases: &[(f32, f32, f32, c_int, c_int, c_int)] = &[
            (0.0, 0.0, 0.0, 0, 0, 0),
            (0.5, 0.25, 0.125, 0, 0, 0),
            (1.5, -2.5, 3.75, 256, 256, 256),
            (-0.1, -0.2, -0.3, 0, 0, 0),
            (12.345, -67.89, 5.0, 64, 64, 64),
            (100.0, 200.0, 300.0, 0, 0, 0),
            (0.001, 0.002, 0.003, 0, 0, 0),
        ];
        for &(x, y, z, xw, yw, zw) in cases {
            let c = c_inner(0, x, y, z, xw, yw, zw, 0, 0.0, 0.0, 0.0, 0);
            let r = r_inner(0, x, y, z, xw, yw, zw, 0, 0.0, 0.0, 0.0, 0);
            assert_f32_bit_eq(c, r, &format!("noise3({x},{y},{z},{xw},{yw},{zw})"));
        }
    }
}

#[test]
fn test_inner_noise3_seed() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let c_inner: Symbol<InnerFn> = c_lib.get(b"inner").unwrap();
        let r_inner: Symbol<InnerFn> = r_lib.get(b"inner").unwrap();

        for seed in [0i32, 1, 42, 100, 200, 255, 256, -1, -100] {
            for &(x, y, z) in &[
                (0.0f32, 0.0, 0.0),
                (1.5, 2.5, 3.5),
                (-0.5, 0.25, 0.125),
                (10.0, 20.0, 30.0),
            ] {
                let c = c_inner(1, x, y, z, 0, 0, 0, seed, 0.0, 0.0, 0.0, 0);
                let r = r_inner(1, x, y, z, 0, 0, 0, seed, 0.0, 0.0, 0.0, 0);
                assert_f32_bit_eq(c, r, &format!("noise3_seed seed={seed} pt=({x},{y},{z})"));
            }
        }
    }
}

#[test]
fn test_inner_ridge_fbm_turb() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let c_inner: Symbol<InnerFn> = c_lib.get(b"inner").unwrap();
        let r_inner: Symbol<InnerFn> = r_lib.get(b"inner").unwrap();

        for &which in &[2i32, 3, 4] {
            for &(x, y, z, lac, gain, off, oct) in &[
                (0.0f32, 0.0, 0.0, 2.0, 0.5, 1.0, 4),
                (1.5, 2.5, 3.5, 2.0, 0.5, 1.0, 6),
                (-1.0, -2.0, -3.0, 2.0, 0.5, 1.0, 8),
                (0.123, 0.456, 0.789, 1.5, 0.6, 0.7, 5),
                (10.0, -5.0, 7.5, 2.5, 0.4, 1.2, 3),
                (0.0, 0.0, 0.0, 2.0, 0.5, 1.0, 0),
                (0.0, 0.0, 0.0, 2.0, 0.5, 1.0, 1),
            ] {
                let c = c_inner(which, x, y, z, 0, 0, 0, 0, lac, gain, off, oct);
                let r = r_inner(which, x, y, z, 0, 0, 0, 0, lac, gain, off, oct);
                assert_f32_bit_eq(
                    c,
                    r,
                    &format!("which={which} pt=({x},{y},{z}) lac={lac} gain={gain} off={off} oct={oct}"),
                );
            }
        }
    }
}

#[test]
fn test_inner_wrap_nonpow2() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let c_inner: Symbol<InnerFn> = c_lib.get(b"inner").unwrap();
        let r_inner: Symbol<InnerFn> = r_lib.get(b"inner").unwrap();

        for seed in [0i32, 1, 7, 42, 100, 255, 256] {
            for &(x, y, z, xw, yw, zw) in &[
                (0.0f32, 0.0, 0.0, 0, 0, 0),
                (0.5, 1.5, 2.5, 7, 11, 13),
                (-0.5, -1.5, -2.5, 7, 11, 13),
                (12.345, -6.789, 4.5, 5, 9, 17),
                (100.0, 200.0, 300.0, 3, 5, 7),
                (0.0, 0.0, 0.0, 1, 1, 1),
            ] {
                let c = c_inner(5, x, y, z, xw, yw, zw, seed, 0.0, 0.0, 0.0, 0);
                let r = r_inner(5, x, y, z, xw, yw, zw, seed, 0.0, 0.0, 0.0, 0);
                assert_f32_bit_eq(
                    c,
                    r,
                    &format!("wrap_nonpow2 seed={seed} pt=({x},{y},{z}) wrap=({xw},{yw},{zw})"),
                );
            }
        }
    }
}

#[test]
fn test_inner_default_nan() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let c_inner: Symbol<InnerFn> = c_lib.get(b"inner").unwrap();
        let r_inner: Symbol<InnerFn> = r_lib.get(b"inner").unwrap();

        for which in [-1i32, 6, 7, 100] {
            let c = c_inner(which, 1.0, 2.0, 3.0, 0, 0, 0, 0, 2.0, 0.5, 1.0, 4);
            let r = r_inner(which, 1.0, 2.0, 3.0, 0, 0, 0, 0, 2.0, 0.5, 1.0, 4);
            assert!(c.is_nan() && r.is_nan(), "default branch should be NaN");
        }
    }
}

#[test]
fn test_perlin_noise3_internal_direct() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let c_fn: Symbol<InternalFn> = c_lib.get(b"stb_perlin_noise3_internal").unwrap();
        let r_fn: Symbol<InternalFn> = r_lib.get(b"stb_perlin_noise3_internal").unwrap();

        for seed in [0u8, 1, 7, 42, 100, 255] {
            for &(x, y, z, xw, yw, zw) in &[
                (0.0f32, 0.0, 0.0, 0, 0, 0),
                (0.5, 1.5, 2.5, 256, 256, 256),
                (-0.5, -1.5, -2.5, 256, 256, 256),
                (12.345, -6.789, 4.5, 64, 64, 64),
                (100.0, 200.0, 300.0, 0, 0, 0),
            ] {
                let c = c_fn(x, y, z, xw, yw, zw, seed);
                let r = r_fn(x, y, z, xw, yw, zw, seed);
                assert_f32_bit_eq(
                    c,
                    r,
                    &format!("internal seed={seed} pt=({x},{y},{z})"),
                );
            }
        }
    }
}

#[test]
fn test_perlin_noise3_direct() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let c_fn: Symbol<Noise3Fn> = c_lib.get(b"stb_perlin_noise3").unwrap();
        let r_fn: Symbol<Noise3Fn> = r_lib.get(b"stb_perlin_noise3").unwrap();

        for &(x, y, z, xw, yw, zw) in &[
            (0.0f32, 0.0, 0.0, 0, 0, 0),
            (0.5, 1.5, 2.5, 256, 256, 256),
            (-0.5, -1.5, -2.5, 256, 256, 256),
            (12.345, -6.789, 4.5, 64, 64, 64),
            (100.0, 200.0, 300.0, 0, 0, 0),
        ] {
            let c = c_fn(x, y, z, xw, yw, zw);
            let r = r_fn(x, y, z, xw, yw, zw);
            assert_f32_bit_eq(c, r, &format!("noise3 ({x},{y},{z})"));
        }
    }
}

#[test]
fn test_perlin_noise3_seed_direct() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let c_fn: Symbol<Noise3SeedFn> = c_lib.get(b"stb_perlin_noise3_seed").unwrap();
        let r_fn: Symbol<Noise3SeedFn> = r_lib.get(b"stb_perlin_noise3_seed").unwrap();

        for seed in [0i32, 1, 42, 100, 255, 256, -1, -100] {
            for &(x, y, z, xw, yw, zw) in &[
                (0.0f32, 0.0, 0.0, 0, 0, 0),
                (0.5, 1.5, 2.5, 256, 256, 256),
                (12.345, -6.789, 4.5, 64, 64, 64),
            ] {
                let c = c_fn(x, y, z, xw, yw, zw, seed);
                let r = r_fn(x, y, z, xw, yw, zw, seed);
                assert_f32_bit_eq(
                    c,
                    r,
                    &format!("noise3_seed seed={seed} pt=({x},{y},{z})"),
                );
            }
        }
    }
}

#[test]
fn test_perlin_ridge_direct() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let c_fn: Symbol<RidgeFn> = c_lib.get(b"stb_perlin_ridge_noise3").unwrap();
        let r_fn: Symbol<RidgeFn> = r_lib.get(b"stb_perlin_ridge_noise3").unwrap();

        for &(x, y, z, lac, gain, off, oct) in &[
            (0.0f32, 0.0, 0.0, 2.0, 0.5, 1.0, 4),
            (1.5, 2.5, 3.5, 2.0, 0.5, 1.0, 6),
            (-1.0, -2.0, -3.0, 2.0, 0.5, 1.0, 8),
            (0.123, 0.456, 0.789, 1.5, 0.6, 0.7, 5),
        ] {
            let c = c_fn(x, y, z, lac, gain, off, oct);
            let r = r_fn(x, y, z, lac, gain, off, oct);
            assert_f32_bit_eq(c, r, &format!("ridge ({x},{y},{z})"));
        }
    }
}

#[test]
fn test_perlin_fbm_direct() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let c_fn: Symbol<FbmFn> = c_lib.get(b"stb_perlin_fbm_noise3").unwrap();
        let r_fn: Symbol<FbmFn> = r_lib.get(b"stb_perlin_fbm_noise3").unwrap();

        for &(x, y, z, lac, gain, oct) in &[
            (0.0f32, 0.0, 0.0, 2.0, 0.5, 4),
            (1.5, 2.5, 3.5, 2.0, 0.5, 6),
            (-1.0, -2.0, -3.0, 2.0, 0.5, 8),
            (0.123, 0.456, 0.789, 1.5, 0.6, 5),
        ] {
            let c = c_fn(x, y, z, lac, gain, oct);
            let r = r_fn(x, y, z, lac, gain, oct);
            assert_f32_bit_eq(c, r, &format!("fbm ({x},{y},{z})"));
        }
    }
}

#[test]
fn test_perlin_turbulence_direct() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let c_fn: Symbol<TurbFn> = c_lib.get(b"stb_perlin_turbulence_noise3").unwrap();
        let r_fn: Symbol<TurbFn> = r_lib.get(b"stb_perlin_turbulence_noise3").unwrap();

        for &(x, y, z, lac, gain, oct) in &[
            (0.0f32, 0.0, 0.0, 2.0, 0.5, 4),
            (1.5, 2.5, 3.5, 2.0, 0.5, 6),
            (-1.0, -2.0, -3.0, 2.0, 0.5, 8),
            (0.123, 0.456, 0.789, 1.5, 0.6, 5),
        ] {
            let c = c_fn(x, y, z, lac, gain, oct);
            let r = r_fn(x, y, z, lac, gain, oct);
            assert_f32_bit_eq(c, r, &format!("turb ({x},{y},{z})"));
        }
    }
}

// Simple xorshift PRNG for deterministic random tests.
struct Xs(u64);
impl Xs {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn f32_range(&mut self, lo: f32, hi: f32) -> f32 {
        let u = (self.next() & 0xffffff) as f32 / (1u32 << 24) as f32;
        lo + (hi - lo) * u
    }
    fn i32_range(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi - lo) as u64;
        lo + (self.next() % span) as i32
    }
}

#[test]
fn test_random_inner_all_branches() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let c_inner: Symbol<InnerFn> = c_lib.get(b"inner").unwrap();
        let r_inner: Symbol<InnerFn> = r_lib.get(b"inner").unwrap();

        let mut rng = Xs(0xdeadbeefcafebabe);
        for which in 0..6 {
            for _ in 0..200 {
                let x = rng.f32_range(-100.0, 100.0);
                let y = rng.f32_range(-100.0, 100.0);
                let z = rng.f32_range(-100.0, 100.0);
                // For wrap-pow2 paths, x_wrap must be 0 or a power of two-ish
                // — but the C code accepts any value; just match it.
                let xw = rng.i32_range(0, 257);
                let yw = rng.i32_range(0, 257);
                let zw = rng.i32_range(0, 257);
                let seed = rng.i32_range(-300, 300);
                let lac = rng.f32_range(1.1, 3.0);
                let gain = rng.f32_range(0.3, 0.8);
                let off = rng.f32_range(0.5, 1.5);
                let oct = rng.i32_range(1, 8);

                // For which=5 (wrap_nonpow2), wrap=0 means default 256; allow but
                // also avoid xw==0 leading to division? It's fine — code handles 0.
                let c = c_inner(which, x, y, z, xw, yw, zw, seed, lac, gain, off, oct);
                let r = r_inner(which, x, y, z, xw, yw, zw, seed, lac, gain, off, oct);
                assert_f32_bit_eq(
                    c,
                    r,
                    &format!(
                        "rand which={which} pt=({x},{y},{z}) wrap=({xw},{yw},{zw}) seed={seed} lac={lac} gain={gain} off={off} oct={oct}"
                    ),
                );
            }
        }
    }
}

#[test]
fn test_perlin_wrap_nonpow2_direct() {
    unsafe {
        let c_lib = Library::new(c_so_path()).expect("load C .so");
        let r_lib = Library::new(rust_so_path()).expect("load Rust .so");
        let c_fn: Symbol<Wrap2Fn> = c_lib.get(b"stb_perlin_noise3_wrap_nonpow2").unwrap();
        let r_fn: Symbol<Wrap2Fn> = r_lib.get(b"stb_perlin_noise3_wrap_nonpow2").unwrap();

        for seed in [0u8, 1, 7, 42, 100, 255] {
            for &(x, y, z, xw, yw, zw) in &[
                (0.0f32, 0.0, 0.0, 0, 0, 0),
                (0.5, 1.5, 2.5, 7, 11, 13),
                (-0.5, -1.5, -2.5, 7, 11, 13),
                (12.345, -6.789, 4.5, 5, 9, 17),
                (100.0, 200.0, 300.0, 3, 5, 7),
                (0.0, 0.0, 0.0, 1, 1, 1),
            ] {
                let c = c_fn(x, y, z, xw, yw, zw, seed);
                let r = r_fn(x, y, z, xw, yw, zw, seed);
                assert_f32_bit_eq(
                    c,
                    r,
                    &format!("wrap_nonpow2 seed={seed} pt=({x},{y},{z}) wrap=({xw},{yw},{zw})"),
                );
            }
        }
    }
}
