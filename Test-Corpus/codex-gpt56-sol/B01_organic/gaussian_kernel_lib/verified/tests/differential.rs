use libloading::Library;
use std::env;
use std::ffi::{c_float, c_int};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::OnceLock;

type GaussianKernel = unsafe extern "C" fn(*mut c_float, c_int, c_float);

static RUST_LIBRARY_PATH: OnceLock<PathBuf> = OnceLock::new();

struct Api {
    _library: Library,
    gaussian_kernel: GaussianKernel,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let gaussian_kernel = unsafe {
            *library
                .get::<GaussianKernel>(b"gaussian_kernel\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to resolve gaussian_kernel in {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            gaussian_kernel,
        }
    }
}

struct Libraries {
    c: Api,
    rust: Api,
}

impl Libraries {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libtranslated_rust.so");
        let rust_path = rust_library_path(&root);
        assert!(
            c_path.is_file(),
            "missing C shared library {}; build c_src first",
            c_path.display()
        );

        unsafe {
            Self {
                c: Api::load(&c_path),
                rust: Api::load(&rust_path),
            }
        }
    }
}

fn rust_library_path(root: &Path) -> PathBuf {
    if let Some(path) = env::var_os("RUST_GAUSSIAN_SO") {
        return PathBuf::from(path);
    }

    RUST_LIBRARY_PATH
        .get_or_init(|| build_and_find_rust_library(root))
        .clone()
}

fn build_and_find_rust_library(root: &Path) -> PathBuf {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new("timeout");
    command.current_dir(root).arg("600").arg(cargo).args([
        "build",
        "--lib",
        "--no-default-features",
        "--features",
        "",
    ]);
    if !cfg!(debug_assertions) {
        command.arg("--release");
    }
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("failed to run cargo build for Rust cdylib: {error}"));
    assert!(status.success(), "cargo build for Rust cdylib failed");

    let target = match env::var_os("CARGO_TARGET_DIR").map(PathBuf::from) {
        Some(path) if path.is_absolute() => path,
        Some(path) => root.join(path),
        None => root.join("target"),
    };
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let profile_dir = target.join(profile);
    let direct = profile_dir.join("libgaussian_kernel_lib.so");
    let deps = profile_dir.join("deps/libgaussian_kernel_lib.so");

    [direct, deps]
        .into_iter()
        .filter(|path| path.is_file())
        .max_by_key(|path| path.metadata().and_then(|meta| meta.modified()).ok())
        .unwrap_or_else(|| {
            panic!(
                "missing Rust shared library under {}; build the cdylib first",
                profile_dir.display()
            )
        })
}

#[derive(Clone)]
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

    fn index(&mut self, length: usize) -> usize {
        (self.next_u64() as usize) % length
    }

    fn unit_f32(&mut self) -> f32 {
        (self.next_u32() as f64 / u32::MAX as f64) as f32
    }

    fn signed(&mut self, magnitude: f32) -> f32 {
        if self.next_u64() & 1 == 0 {
            magnitude
        } else {
            -magnitude
        }
    }
}

fn no_sum_radius(rng: &mut Rng) -> f32 {
    match rng.index(5) {
        0 => 0.0,
        1 => -0.0,
        2 => f32::from_bits(0x7fc0_0000 | (rng.next_u32() & 0x003f_ffff)),
        3 => f32::from_bits(1 + (rng.next_u32() & 0x0000_03ff)),
        _ => -f32::from_bits(1 + (rng.next_u32() & 0x0000_03ff)),
    }
}

fn usable_radius(rng: &mut Rng) -> f32 {
    let magnitude = 0.75 + 100.0 * rng.unit_f32();
    rng.signed(magnitude)
}

fn narrow_radius(rng: &mut Rng) -> f32 {
    let magnitude = 0.01 + 0.55 * rng.unit_f32();
    rng.signed(magnitude)
}

fn broad_radius(rng: &mut Rng, size: i32) -> f32 {
    let hsize = (size / 2).max(1) as f32;
    let magnitude = hsize * (1.0 + 7.0 * rng.unit_f32()) + 1.0;
    rng.signed(magnitude)
}

fn infinite_radius(rng: &mut Rng) -> f32 {
    rng.signed(f32::INFINITY)
}

fn written_elements(size: i32) -> usize {
    let hsize = size / 2;
    if -hsize > hsize {
        0
    } else {
        (2_i64 * i64::from(hsize) + 1) as usize
    }
}

fn initial_buffer(rng: &mut Rng, size: i32) -> Vec<f32> {
    let length = written_elements(size).max(1) + 8;
    (0..length)
        .map(|_| f32::from_bits(rng.next_u32()))
        .collect()
}

fn assert_case(libraries: &Libraries, row: &str, iteration: usize, size: i32, radius: f32) {
    let seed = 0xd1ff_e000_0000_0000
        ^ (size as u32 as u64).rotate_left(17)
        ^ u64::from(radius.to_bits())
        ^ iteration as u64;
    let mut rng = Rng::new(seed);
    let initial = initial_buffer(&mut rng, size);
    let mut c_output = initial.clone();
    let mut rust_output = initial;

    unsafe {
        (libraries.c.gaussian_kernel)(c_output.as_mut_ptr(), size, radius);
        (libraries.rust.gaussian_kernel)(rust_output.as_mut_ptr(), size, radius);
    }

    let c_bits: Vec<u32> = c_output.iter().map(|value| value.to_bits()).collect();
    let rust_bits: Vec<u32> = rust_output.iter().map(|value| value.to_bits()).collect();
    assert_eq!(
        c_bits,
        rust_bits,
        "{row} diverged at iteration {iteration}, size={size}, radius={radius:?} \
         (bits=0x{:08x})",
        radius.to_bits()
    );
}

fn assert_null_no_access_case(libraries: &Libraries, row: &str, size: i32) {
    assert_eq!(row, "G1");
    unsafe {
        (libraries.c.gaussian_kernel)(ptr::null_mut(), size, 1.0);
        (libraries.rust.gaussian_kernel)(ptr::null_mut(), size, 1.0);
    }
}

fn random_odd_size(rng: &mut Rng) -> i32 {
    3 + 2 * (rng.next_u32() % 63) as i32
}

fn random_even_size(rng: &mut Rng) -> i32 {
    2 + 2 * (rng.next_u32() % 64) as i32
}

#[test]
fn config_c1_skipped_negative_sizes() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc100_0000_0000_0001);
    for iteration in 0..96 {
        let size = -2 - (rng.next_u32() % 100_000) as i32;
        let radius = f32::from_bits(rng.next_u32());
        assert_case(&libraries, "C1", iteration, size, radius);
    }
}

#[test]
fn configs_c2_c3_negative_one() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc203_0000_0000_0001);
    for iteration in 0..64 {
        let radius = no_sum_radius(&mut rng);
        assert_case(&libraries, "C2", iteration, -1, radius);
    }
    for iteration in 0..64 {
        let radius = if iteration % 4 == 0 {
            infinite_radius(&mut rng)
        } else {
            usable_radius(&mut rng)
        };
        assert_case(&libraries, "C3", iteration, -1, radius);
    }
}

#[test]
fn configs_c4_c5_zero_size() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc405_0000_0000_0001);
    for iteration in 0..64 {
        let radius = no_sum_radius(&mut rng);
        assert_case(&libraries, "C4", iteration, 0, radius);
    }
    for iteration in 0..64 {
        let radius = if iteration % 4 == 0 {
            infinite_radius(&mut rng)
        } else {
            usable_radius(&mut rng)
        };
        assert_case(&libraries, "C5", iteration, 0, radius);
    }
}

#[test]
fn configs_c6_c7_one_element() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc607_0000_0000_0001);
    for iteration in 0..64 {
        let radius = no_sum_radius(&mut rng);
        assert_case(&libraries, "C6", iteration, 1, radius);
    }
    for iteration in 0..64 {
        let radius = if iteration % 4 == 0 {
            infinite_radius(&mut rng)
        } else {
            usable_radius(&mut rng)
        };
        assert_case(&libraries, "C7", iteration, 1, radius);
    }
}

#[test]
fn config_c8_odd_no_sum() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc800_0000_0000_0001);
    for iteration in 0..96 {
        let size = random_odd_size(&mut rng);
        let radius = no_sum_radius(&mut rng);
        assert_case(&libraries, "C8", iteration, size, radius);
    }
}

#[test]
fn config_c9_odd_narrow() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc900_0000_0000_0001);
    for iteration in 0..96 {
        let size = random_odd_size(&mut rng);
        let radius = narrow_radius(&mut rng);
        assert_case(&libraries, "C9", iteration, size, radius);
    }
}

#[test]
fn config_c10_odd_broad() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc100_0000_0000_0010);
    for iteration in 0..96 {
        let size = random_odd_size(&mut rng);
        let radius = broad_radius(&mut rng, size);
        assert_case(&libraries, "C10", iteration, size, radius);
    }
}

#[test]
fn config_c11_odd_infinite() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc110_0000_0000_0011);
    for iteration in 0..96 {
        let size = random_odd_size(&mut rng);
        let radius = infinite_radius(&mut rng);
        assert_case(&libraries, "C11", iteration, size, radius);
    }
}

#[test]
fn config_c12_even_no_sum() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc120_0000_0000_0012);
    for iteration in 0..96 {
        let size = random_even_size(&mut rng);
        let radius = no_sum_radius(&mut rng);
        assert_case(&libraries, "C12", iteration, size, radius);
    }
}

#[test]
fn config_c13_even_narrow() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc130_0000_0000_0013);
    for iteration in 0..96 {
        let size = random_even_size(&mut rng);
        let radius = narrow_radius(&mut rng);
        assert_case(&libraries, "C13", iteration, size, radius);
    }
}

#[test]
fn config_c14_even_broad() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc140_0000_0000_0014);
    for iteration in 0..96 {
        let size = random_even_size(&mut rng);
        let radius = broad_radius(&mut rng, size);
        assert_case(&libraries, "C14", iteration, size, radius);
    }
}

#[test]
fn config_c15_even_infinite() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc150_0000_0000_0015);
    for iteration in 0..96 {
        let size = random_even_size(&mut rng);
        let radius = infinite_radius(&mut rng);
        assert_case(&libraries, "C15", iteration, size, radius);
    }
}

#[test]
fn config_c16_oversized_even() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0xc160_0000_0000_0016);
    for iteration in 0..16 {
        let radius = narrow_radius(&mut rng);
        assert_case(&libraries, "C16", iteration, 65_536, radius);
    }
}

#[test]
fn generic_ffi_boundaries_g1_g2_g3() {
    let libraries = Libraries::load();

    for size in [-2, -3, i32::MIN] {
        assert_null_no_access_case(&libraries, "G1", size);
    }

    let mut rng = Rng::new(0x600d_0000_0000_0001);
    for iteration in 0..32 {
        assert_case(
            &libraries,
            "G2",
            iteration,
            0,
            f32::from_bits(rng.next_u32()),
        );
    }
    for iteration in 0..16 {
        let radius = narrow_radius(&mut rng);
        assert_case(&libraries, "G3", iteration, 65_536, radius);
    }
}
