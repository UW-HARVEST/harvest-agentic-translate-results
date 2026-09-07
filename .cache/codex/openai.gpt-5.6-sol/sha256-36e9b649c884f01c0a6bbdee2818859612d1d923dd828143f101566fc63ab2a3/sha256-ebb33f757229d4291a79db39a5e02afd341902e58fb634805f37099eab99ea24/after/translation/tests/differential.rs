use libloading::Library;
use std::ffi::{c_float, c_int};
use std::path::{Path, PathBuf};
use std::ptr;

type GaussianKernel = unsafe extern "C" fn(*mut c_float, c_int, c_float);

const ITERATIONS: usize = 128;

struct Kernels {
    _c_library: Library,
    _rust_library: Library,
    c: GaussianKernel,
    rust: GaussianKernel,
}

impl Kernels {
    fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = find_c_library(&manifest.join("../c_src/build"));
        let rust_path = manifest.join("target/release/libgaussian_kernel_lib.so");
        assert!(
            rust_path.is_file(),
            "missing Rust cdylib: {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("load {}: {error}", rust_path.display()));
            let c = *c_library
                .get::<GaussianKernel>(b"gaussian_kernel\0")
                .expect("C gaussian_kernel export");
            let rust = *rust_library
                .get::<GaussianKernel>(b"gaussian_kernel\0")
                .expect("Rust gaussian_kernel export");
            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c,
                rust,
            }
        }
    }

    fn compare(&self, size: i32, radius: f32, rng: &mut Rng) -> Vec<u32> {
        let generated = generated_len(size);
        let len = generated.max(1) + 8;
        let initial: Vec<f32> = (0..len).map(|_| f32::from_bits(rng.next_u32())).collect();
        let mut c_output = initial.clone();
        let mut rust_output = initial;

        unsafe {
            (self.c)(c_output.as_mut_ptr(), size, radius);
            (self.rust)(rust_output.as_mut_ptr(), size, radius);
        }

        let c_bits: Vec<u32> = c_output.iter().map(|value| value.to_bits()).collect();
        let rust_bits: Vec<u32> = rust_output.iter().map(|value| value.to_bits()).collect();
        assert_eq!(
            c_bits,
            rust_bits,
            "byte divergence for size={size}, radius={radius:?} ({:#010x})",
            radius.to_bits()
        );
        c_bits
    }
}

fn find_c_library(build_dir: &Path) -> PathBuf {
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(build_dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", build_dir.display()))
        .map(|entry| entry.expect("C build entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("lib") && name.ends_with(".so"))
        })
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected one C shared library in {}: {candidates:?}",
        build_dir.display()
    );
    candidates.remove(0)
}

fn generated_len(size: i32) -> usize {
    if size <= -2 {
        0
    } else {
        let half = size / 2;
        (2_i64 * i64::from(half) + 1) as usize
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16_777_216.0
    }

    fn sign(&mut self) -> f32 {
        if self.next_u32() & 1 == 0 { 1.0 } else { -1.0 }
    }

    fn odd_size(&mut self, minimum_half: i32) -> i32 {
        let half = minimum_half + (self.next_u32() % 63) as i32;
        2 * half + 1
    }

    fn even_size(&mut self, minimum_half: i32) -> i32 {
        let half = minimum_half + (self.next_u32() % 63) as i32;
        2 * half
    }

    fn finite_center_radius(&mut self) -> f32 {
        self.sign() * (0.01 + self.unit() * 0.58)
    }

    fn tiny_overflow_radius(&mut self) -> f32 {
        let magnitude = 1 + self.next_u32() % 1_000_000;
        f32::from_bits(magnitude | (self.next_u32() & 0x8000_0000))
    }

    fn nan(&mut self) -> f32 {
        let payload = 1 + (self.next_u32() & 0x007f_ffff);
        f32::from_bits(0x7f80_0000 | payload | (self.next_u32() & 0x8000_0000))
    }
}

fn positive_count(bits: &[u32], generated: usize) -> usize {
    bits[..generated]
        .iter()
        .filter(|bits| f32::from_bits(**bits) > 0.0)
        .count()
}

fn assert_all_zero(bits: &[u32], generated: usize) {
    assert!(
        bits[..generated]
            .iter()
            .all(|bits| f32::from_bits(*bits) == 0.0),
        "expected all generated coefficients to be zero"
    );
}

#[test]
fn phase_b_c1_negative_sizes_skip_both_loops() {
    let kernels = Kernels::load();
    let mut rng = Rng::new(0xc1c1_c1c1_0000_0001);
    for _ in 0..ITERATIONS {
        let size = -2 - (rng.next_u32() % 1_000_000) as i32;
        kernels.compare(size, f32::from_bits(rng.next_u32()), &mut rng);
    }
}

fn compare_single_raw_positive(size: i32, seed: u64) {
    let kernels = Kernels::load();
    let mut rng = Rng::new(seed);
    for iteration in 0..ITERATIONS {
        let radius = if iteration & 7 == 0 {
            rng.sign() * f32::INFINITY
        } else {
            rng.sign() * (0.001 + rng.unit() * 10_000.0)
        };
        let bits = kernels.compare(size, radius, &mut rng);
        assert_eq!(positive_count(&bits, 1), 1);
    }
}

fn compare_single_zero(size: i32, seed: u64) {
    let kernels = Kernels::load();
    let mut rng = Rng::new(seed);
    for iteration in 0..ITERATIONS {
        let radius = match iteration % 4 {
            0 => 0.0,
            1 => -0.0,
            2 => rng.tiny_overflow_radius(),
            _ => rng.nan(),
        };
        let bits = kernels.compare(size, radius, &mut rng);
        assert_all_zero(&bits, 1);
    }
}

#[test]
fn phase_b_c2_size_minus_one_raw_positive() {
    compare_single_raw_positive(-1, 0xc2c2_c2c2_0000_0002);
}

#[test]
fn phase_b_c3_size_minus_one_zero_classes() {
    compare_single_zero(-1, 0xc3c3_c3c3_0000_0003);
}

#[test]
fn phase_b_c4_size_zero_raw_positive() {
    compare_single_raw_positive(0, 0xc4c4_c4c4_0000_0004);
}

#[test]
fn phase_b_c5_size_zero_zero_classes() {
    compare_single_zero(0, 0xc5c5_c5c5_0000_0005);
}

#[test]
fn phase_b_c6_size_one_normalized_positive() {
    compare_single_raw_positive(1, 0xc6c6_c6c6_0000_0006);
}

#[test]
fn phase_b_c7_size_one_zero_classes() {
    compare_single_zero(1, 0xc7c7_c7c7_0000_0007);
}

#[derive(Clone, Copy)]
enum Parity {
    Odd,
    Even,
}

fn random_size(rng: &mut Rng, parity: Parity, minimum_half: i32) -> i32 {
    match parity {
        Parity::Odd => rng.odd_size(minimum_half),
        Parity::Even => rng.even_size(minimum_half),
    }
}

fn compare_broad(parity: Parity, seed: u64) {
    let kernels = Kernels::load();
    let mut rng = Rng::new(seed);
    for _ in 0..ITERATIONS {
        let size = random_size(&mut rng, parity, 1);
        let half = size / 2;
        let radius = rng.sign() * (half as f32 + 1.0 + rng.unit() * 1_000.0);
        let bits = kernels.compare(size, radius, &mut rng);
        assert_eq!(
            positive_count(&bits, generated_len(size)),
            generated_len(size)
        );
    }
}

fn compare_mixed(parity: Parity, seed: u64) {
    let kernels = Kernels::load();
    let mut rng = Rng::new(seed);
    for _ in 0..ITERATIONS {
        let size = random_size(&mut rng, parity, 2);
        let half = size / 2;
        let upper = half as f32 / 1.6;
        let radius = rng.sign() * (0.75 + rng.unit() * (upper - 0.75));
        let bits = kernels.compare(size, radius, &mut rng);
        let positives = positive_count(&bits, generated_len(size));
        assert!(positives > 1 && positives < generated_len(size));
    }
}

fn compare_center_only(parity: Parity, seed: u64) {
    let kernels = Kernels::load();
    let mut rng = Rng::new(seed);
    for _ in 0..ITERATIONS {
        let size = random_size(&mut rng, parity, 1);
        let bits = kernels.compare(size, rng.finite_center_radius(), &mut rng);
        assert_eq!(positive_count(&bits, generated_len(size)), 1);
    }
}

fn compare_tiny(parity: Parity, seed: u64) {
    let kernels = Kernels::load();
    let mut rng = Rng::new(seed);
    for _ in 0..ITERATIONS {
        let size = random_size(&mut rng, parity, 1);
        let bits = kernels.compare(size, rng.tiny_overflow_radius(), &mut rng);
        assert_all_zero(&bits, generated_len(size));
    }
}

fn compare_zero(parity: Parity, seed: u64) {
    let kernels = Kernels::load();
    let mut rng = Rng::new(seed);
    for iteration in 0..ITERATIONS {
        let size = random_size(&mut rng, parity, 1);
        let radius = if iteration & 1 == 0 { 0.0 } else { -0.0 };
        let bits = kernels.compare(size, radius, &mut rng);
        assert_all_zero(&bits, generated_len(size));
    }
}

fn compare_nan(parity: Parity, seed: u64) {
    let kernels = Kernels::load();
    let mut rng = Rng::new(seed);
    for _ in 0..ITERATIONS {
        let size = random_size(&mut rng, parity, 1);
        let bits = kernels.compare(size, rng.nan(), &mut rng);
        assert_all_zero(&bits, generated_len(size));
    }
}

fn compare_infinite(parity: Parity, seed: u64) {
    let kernels = Kernels::load();
    let mut rng = Rng::new(seed);
    for iteration in 0..ITERATIONS {
        let size = random_size(&mut rng, parity, 1);
        let radius = if iteration & 1 == 0 {
            f32::INFINITY
        } else {
            f32::NEG_INFINITY
        };
        let bits = kernels.compare(size, radius, &mut rng);
        assert_eq!(
            positive_count(&bits, generated_len(size)),
            generated_len(size)
        );
    }
}

#[test]
fn phase_b_c8_odd_all_positive() {
    compare_broad(Parity::Odd, 0xc8c8_c8c8_0000_0008);
}

#[test]
fn phase_b_c9_odd_mixed_clipping() {
    compare_mixed(Parity::Odd, 0xc9c9_c9c9_0000_0009);
}

#[test]
fn phase_b_c10_odd_center_only() {
    compare_center_only(Parity::Odd, 0xcaca_caca_0000_0010);
}

#[test]
fn phase_b_c11_odd_tiny_radius_overflow() {
    compare_tiny(Parity::Odd, 0xcbcb_cbcb_0000_0011);
}

#[test]
fn phase_b_c12_odd_signed_zero_radius() {
    compare_zero(Parity::Odd, 0xcccc_cccc_0000_0012);
}

#[test]
fn phase_b_c13_odd_nan_radius() {
    compare_nan(Parity::Odd, 0xcdcd_cdcd_0000_0013);
}

#[test]
fn phase_b_c14_odd_infinite_radius() {
    compare_infinite(Parity::Odd, 0xcece_cece_0000_0014);
}

#[test]
fn phase_b_c15_even_all_positive_and_extra_raw() {
    compare_broad(Parity::Even, 0xcfcf_cfcf_0000_0015);
}

#[test]
fn phase_b_c16_even_mixed_clipping_and_extra_raw() {
    compare_mixed(Parity::Even, 0xd0d0_d0d0_0000_0016);
}

#[test]
fn phase_b_c17_even_center_only() {
    compare_center_only(Parity::Even, 0xd1d1_d1d1_0000_0017);
}

#[test]
fn phase_b_c18_even_tiny_radius_overflow() {
    compare_tiny(Parity::Even, 0xd2d2_d2d2_0000_0018);
}

#[test]
fn phase_b_c19_even_signed_zero_radius() {
    compare_zero(Parity::Even, 0xd3d3_d3d3_0000_0019);
}

#[test]
fn phase_b_c20_even_nan_radius() {
    compare_nan(Parity::Even, 0xd4d4_d4d4_0000_0020);
}

#[test]
fn phase_b_c21_even_infinite_radius_and_extra_raw() {
    compare_infinite(Parity::Even, 0xd5d5_d5d5_0000_0021);
}

#[test]
fn phase_c_g1_null_pointer_is_safe_when_loops_are_skipped() {
    let kernels = Kernels::load();
    for size in [-2, -3, -10_000, i32::MIN] {
        unsafe {
            (kernels.c)(ptr::null_mut(), size, f32::NAN);
            (kernels.rust)(ptr::null_mut(), size, f32::NAN);
        }
    }
}

#[test]
fn phase_c_g2_zero_size_writes_one_coefficient() {
    let kernels = Kernels::load();
    let mut rng = Rng::new(0xe2e2_e2e2_0000_0002);
    for _ in 0..ITERATIONS {
        kernels.compare(0, f32::from_bits(rng.next_u32()), &mut rng);
    }
}

#[test]
fn phase_c_g3_oversized_allocated_lengths() {
    let kernels = Kernels::load();
    let mut rng = Rng::new(0xe3e3_e3e3_0000_0003);
    for size in [16_384, 16_385, 65_536, 65_537] {
        for radius in [0.0, -0.0, 0.25, -3.5, 1_000_000.0, f32::INFINITY, f32::NAN] {
            kernels.compare(size, radius, &mut rng);
        }
    }
}
