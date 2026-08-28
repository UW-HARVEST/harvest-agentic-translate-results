use libloading::Library;
use std::env;
use std::ffi::{c_float, c_int};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::OnceLock;

type GaussianKernel = unsafe extern "C" fn(*mut c_float, c_int, c_float);

struct Kernels {
    _c_library: Library,
    _rust_library: Library,
    c: GaussianKernel,
    rust: GaussianKernel,
}

// libloading keeps the libraries open while copied function pointers are used.
unsafe impl Send for Kernels {}
unsafe impl Sync for Kernels {}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libharvest-work-Urz3Dq.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libgaussian_kernel_lib.so")
}

fn load_kernel(path: &Path) -> (Library, GaussianKernel) {
    let library = unsafe { Library::new(path) }
        .unwrap_or_else(|error| panic!("load {}: {error}", path.display()));
    let kernel = unsafe {
        *library
            .get::<GaussianKernel>(b"gaussian_kernel\0")
            .unwrap_or_else(|error| panic!("load gaussian_kernel from {}: {error}", path.display()))
    };
    (library, kernel)
}

fn kernels() -> &'static Kernels {
    static KERNELS: OnceLock<Kernels> = OnceLock::new();
    KERNELS.get_or_init(|| {
        let (c_library, c) = load_kernel(&c_library_path());
        let (rust_library, rust) = load_kernel(&rust_library_path());
        Kernels {
            _c_library: c_library,
            _rust_library: rust_library,
            c,
            rust,
        }
    })
}

#[derive(Clone, Copy)]
enum SizeClass {
    NoIterations,
    MinusOne,
    Zero,
    One,
    PositiveOdd,
    PositiveEven,
}

#[derive(Clone, Copy)]
enum RadiusClass {
    Any,
    FiniteRs,
    Infinity,
    Zero,
    Tiny,
    Nan,
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
}

fn size_for(class: SizeClass, rng: &mut Rng, iteration: usize) -> i32 {
    match class {
        SizeClass::NoIterations => {
            const BOUNDARIES: [i32; 8] = [
                i32::MIN,
                i32::MIN + 1,
                -1_000_000,
                -65_536,
                -257,
                -3,
                -2,
                -100,
            ];
            if iteration < BOUNDARIES.len() {
                BOUNDARIES[iteration]
            } else {
                -2 - (rng.next_u32() % (i32::MAX as u32 - 1)) as i32
            }
        }
        SizeClass::MinusOne => -1,
        SizeClass::Zero => 0,
        SizeClass::One => 1,
        SizeClass::PositiveOdd => 3 + 2 * (rng.next_u32() % 63) as i32,
        SizeClass::PositiveEven => 2 + 2 * (rng.next_u32() % 64) as i32,
    }
}

fn radius_for(class: RadiusClass, rng: &mut Rng, iteration: usize) -> f32 {
    match class {
        RadiusClass::Any => f32::from_bits(rng.next_u32()),
        RadiusClass::FiniteRs => {
            let sign = rng.next_u32() & 0x8000_0000;
            let exponent = 1 + rng.next_u32() % 254;
            let fraction = rng.next_u32() & 0x007f_ffff;
            f32::from_bits(sign | (exponent << 23) | fraction)
        }
        RadiusClass::Infinity => f32::from_bits(if iteration % 2 == 0 {
            0x7f80_0000
        } else {
            0xff80_0000
        }),
        RadiusClass::Zero => f32::from_bits(if iteration % 2 == 0 { 0 } else { 0x8000_0000 }),
        RadiusClass::Tiny => {
            let sign = rng.next_u32() & 0x8000_0000;
            let magnitude = 1 + rng.next_u32() % 1_000_000;
            f32::from_bits(sign | magnitude)
        }
        RadiusClass::Nan => {
            let sign = rng.next_u32() & 0x8000_0000;
            let payload = 1 + rng.next_u32() % 0x007f_ffff;
            f32::from_bits(sign | 0x7f80_0000 | payload)
        }
    }
}

fn generated_elements(size: i32) -> usize {
    if size <= -2 {
        0
    } else if size <= 0 {
        1
    } else {
        (2 * (size / 2) + 1) as usize
    }
}

fn run_config_row(row: u32, size_class: SizeClass, radius_class: RadiusClass) {
    const ITERATIONS: usize = 128;
    let mut rng = Rng::new(0x4d59_5df4_d0f3_3173 ^ u64::from(row));
    let kernels = kernels();

    for iteration in 0..ITERATIONS {
        let size = size_for(size_class, &mut rng, iteration);
        let radius = radius_for(radius_class, &mut rng, iteration);
        let payload_len = generated_elements(size).max(1);
        let mut c_buffer: Vec<f32> = (0..payload_len + 4)
            .map(|_| f32::from_bits(rng.next_u32()))
            .collect();
        let mut rust_buffer = c_buffer.clone();

        unsafe {
            (kernels.c)(c_buffer.as_mut_ptr().add(2), size, radius);
            (kernels.rust)(rust_buffer.as_mut_ptr().add(2), size, radius);
        }

        let c_bits: Vec<u32> = c_buffer.iter().map(|value| value.to_bits()).collect();
        let rust_bits: Vec<u32> = rust_buffer.iter().map(|value| value.to_bits()).collect();
        assert_eq!(
            c_bits,
            rust_bits,
            "CONFIGS.md row {row}, iteration {iteration}, size {size}, radius bits {:#010x}",
            radius.to_bits()
        );
    }
}

#[test]
fn config_01_no_iterations_any_radius() {
    run_config_row(1, SizeClass::NoIterations, RadiusClass::Any);
}

#[test]
fn config_02_minus_one_finite_rs() {
    run_config_row(2, SizeClass::MinusOne, RadiusClass::FiniteRs);
}

#[test]
fn config_03_minus_one_infinity() {
    run_config_row(3, SizeClass::MinusOne, RadiusClass::Infinity);
}

#[test]
fn config_04_minus_one_zero() {
    run_config_row(4, SizeClass::MinusOne, RadiusClass::Zero);
}

#[test]
fn config_05_minus_one_tiny() {
    run_config_row(5, SizeClass::MinusOne, RadiusClass::Tiny);
}

#[test]
fn config_06_minus_one_nan() {
    run_config_row(6, SizeClass::MinusOne, RadiusClass::Nan);
}

#[test]
fn config_07_zero_finite_rs() {
    run_config_row(7, SizeClass::Zero, RadiusClass::FiniteRs);
}

#[test]
fn config_08_zero_infinity() {
    run_config_row(8, SizeClass::Zero, RadiusClass::Infinity);
}

#[test]
fn config_09_zero_zero() {
    run_config_row(9, SizeClass::Zero, RadiusClass::Zero);
}

#[test]
fn config_10_zero_tiny() {
    run_config_row(10, SizeClass::Zero, RadiusClass::Tiny);
}

#[test]
fn config_11_zero_nan() {
    run_config_row(11, SizeClass::Zero, RadiusClass::Nan);
}

#[test]
fn config_12_one_finite_rs() {
    run_config_row(12, SizeClass::One, RadiusClass::FiniteRs);
}

#[test]
fn config_13_one_infinity() {
    run_config_row(13, SizeClass::One, RadiusClass::Infinity);
}

#[test]
fn config_14_one_zero() {
    run_config_row(14, SizeClass::One, RadiusClass::Zero);
}

#[test]
fn config_15_one_tiny() {
    run_config_row(15, SizeClass::One, RadiusClass::Tiny);
}

#[test]
fn config_16_one_nan() {
    run_config_row(16, SizeClass::One, RadiusClass::Nan);
}

#[test]
fn config_17_positive_odd_finite_rs() {
    run_config_row(17, SizeClass::PositiveOdd, RadiusClass::FiniteRs);
}

#[test]
fn config_18_positive_odd_infinity() {
    run_config_row(18, SizeClass::PositiveOdd, RadiusClass::Infinity);
}

#[test]
fn config_19_positive_odd_zero() {
    run_config_row(19, SizeClass::PositiveOdd, RadiusClass::Zero);
}

#[test]
fn config_20_positive_odd_tiny() {
    run_config_row(20, SizeClass::PositiveOdd, RadiusClass::Tiny);
}

#[test]
fn config_21_positive_odd_nan() {
    run_config_row(21, SizeClass::PositiveOdd, RadiusClass::Nan);
}

#[test]
fn config_22_positive_even_finite_rs() {
    run_config_row(22, SizeClass::PositiveEven, RadiusClass::FiniteRs);
}

#[test]
fn config_23_positive_even_infinity() {
    run_config_row(23, SizeClass::PositiveEven, RadiusClass::Infinity);
}

#[test]
fn config_24_positive_even_zero() {
    run_config_row(24, SizeClass::PositiveEven, RadiusClass::Zero);
}

#[test]
fn config_25_positive_even_tiny() {
    run_config_row(25, SizeClass::PositiveEven, RadiusClass::Tiny);
}

#[test]
fn config_26_positive_even_nan() {
    run_config_row(26, SizeClass::PositiveEven, RadiusClass::Nan);
}

fn run_child(library: &str, size: i32) -> ExitStatus {
    Command::new(env::current_exe().expect("current test executable"))
        .args(["--exact", "ffi_boundary_child", "--nocapture"])
        .env("GAUSSIAN_KERNEL_CHILD_LIBRARY", library)
        .env("GAUSSIAN_KERNEL_CHILD_SIZE", size.to_string())
        .status()
        .expect("run boundary child")
}

#[test]
fn ffi_boundary_child() {
    let Ok(library_name) = env::var("GAUSSIAN_KERNEL_CHILD_LIBRARY") else {
        return;
    };
    let size: i32 = env::var("GAUSSIAN_KERNEL_CHILD_SIZE")
        .expect("child size")
        .parse()
        .expect("integer child size");
    let path = match library_name.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown child library {other}"),
    };
    let (_library, kernel) = load_kernel(&path);
    unsafe {
        kernel(std::ptr::null_mut(), size, 1.0);
    }
}

fn assert_same_process_result(size: i32, expect_success: bool) {
    let c_status = run_child("c", size);
    let rust_status = run_child("rust", size);
    assert_eq!(
        c_status.success(),
        expect_success,
        "unexpected C process result for size {size}: {c_status}"
    );
    assert_eq!(
        rust_status.success(),
        expect_success,
        "unexpected Rust process result for size {size}: {rust_status}"
    );

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            c_status.signal(),
            rust_status.signal(),
            "different terminating signals for size {size}: C={c_status}, Rust={rust_status}"
        );
    }
}

#[test]
fn boundary_b1_null_no_dereference() {
    for size in [i32::MIN, i32::MIN + 1, -1_000_000, -3, -2] {
        assert_same_process_result(size, true);
    }
}

#[test]
fn boundary_b2_null_dereference() {
    for size in [-1, 0, 1, 2, 127] {
        assert_same_process_result(size, false);
    }
}

#[test]
fn boundary_b3_zero_size_writes_one() {
    run_config_row(7, SizeClass::Zero, RadiusClass::FiniteRs);
}

#[test]
fn boundary_b4_minus_one_writes_one() {
    run_config_row(2, SizeClass::MinusOne, RadiusClass::FiniteRs);
}

#[test]
fn boundary_b5_oversized_positive_size() {
    assert_same_process_result(i32::MAX, false);
}
