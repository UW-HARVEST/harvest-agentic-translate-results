use libloading::Library;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type HslToRgb = unsafe extern "C" fn(*mut f32, *const f32);

const RANDOM_CASES: usize = 2_048;

struct Implementations {
    c_fn: HslToRgb,
    rust_fn: HslToRgb,
    _c_library: Library,
    _rust_library: Library,
}

impl Implementations {
    fn load() -> Self {
        let c_library = load_library(&c_library_path());
        let rust_library = load_library(&rust_library_path());
        let c_fn = load_hsl_to_rgb(&c_library);
        let rust_fn = load_hsl_to_rgb(&rust_library);

        Self {
            c_fn,
            rust_fn,
            _c_library: c_library,
            _rust_library: rust_library,
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }

    fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    fn nonzero_f32(&mut self) -> f32 {
        let value = self.any_f32();
        if value == 0.0 { 1.0 } else { value }
    }

    fn in_range(&mut self, low: f32, high: f32) -> f32 {
        let fraction = (self.next_u32() as f64) / ((u32::MAX as f64) + 1.0);
        let value = low + ((high - low) as f64 * fraction) as f32;
        value.clamp(low, below(high))
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir()
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir().join("target"));
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    target_dir
        .join(profile)
        .join("deps")
        .join("libhsl_to_rgb_lib.so")
}

fn load_library(path: &Path) -> Library {
    assert!(
        path.is_file(),
        "shared library does not exist: {}",
        path.display()
    );
    unsafe { Library::new(path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()))
}

fn load_hsl_to_rgb(library: &Library) -> HslToRgb {
    unsafe { *library.get::<HslToRgb>(b"hsl_to_rgb\0").unwrap() }
}

fn output_bytes(function: HslToRgb, input: [f32; 3]) -> [u8; 12] {
    let mut output = [
        f32::from_bits(0x7fc0_0001),
        f32::from_bits(0x7fc0_0002),
        f32::from_bits(0x7fc0_0003),
    ];
    unsafe { function(output.as_mut_ptr(), input.as_ptr()) };
    floats_to_bytes(output)
}

fn in_place_output_bytes(function: HslToRgb, input: [f32; 3]) -> [u8; 12] {
    let mut buffer = input;
    unsafe { function(buffer.as_mut_ptr(), buffer.as_ptr()) };
    floats_to_bytes(buffer)
}

fn floats_to_bytes(values: [f32; 3]) -> [u8; 12] {
    let mut bytes = [0; 12];
    for (index, value) in values.into_iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

fn assert_case(implementations: &Implementations, row: usize, input: [f32; 3]) {
    let c_output = output_bytes(implementations.c_fn, input);
    let rust_output = output_bytes(implementations.rust_fn, input);
    assert_eq!(
        rust_output,
        c_output,
        "CONFIGS.md row {row}, separate buffers, input bits={:08x?}",
        input.map(f32::to_bits)
    );

    let c_in_place = in_place_output_bytes(implementations.c_fn, input);
    let rust_in_place = in_place_output_bytes(implementations.rust_fn, input);
    assert_eq!(
        rust_in_place,
        c_in_place,
        "CONFIGS.md row {row}, in-place buffer, input bits={:08x?}",
        input.map(f32::to_bits)
    );
}

fn run_cases(
    row: usize,
    seed: u64,
    fixed: &[[f32; 3]],
    mut random_input: impl FnMut(&mut Rng) -> [f32; 3],
) {
    let implementations = Implementations::load();
    for &input in fixed {
        assert_case(&implementations, row, input);
    }

    let mut rng = Rng::new(seed);
    for _ in 0..RANDOM_CASES {
        assert_case(&implementations, row, random_input(&mut rng));
    }
}

fn below(value: f32) -> f32 {
    f32::from_bits(value.to_bits() - 1)
}

#[test]
fn config_01_achromatic() {
    let fixed = [
        [0.0, 0.0, 0.0],
        [-0.0, -0.0, -0.0],
        [f32::NAN, 0.0, f32::from_bits(0x7fa1_2345)],
        [f32::INFINITY, -0.0, f32::NEG_INFINITY],
    ];
    let mut negative_zero = false;
    run_cases(1, 0x0101_0101_0101_0101, &fixed, move |rng| {
        negative_zero = !negative_zero;
        [
            rng.any_f32(),
            if negative_zero { -0.0 } else { 0.0 },
            rng.any_f32(),
        ]
    });
}

#[test]
fn config_02_hue_0_to_60() {
    let fixed = [
        [0.0, 1.0, 0.5],
        [-0.0, -1.0, -0.0],
        [below(60.0), f32::INFINITY, f32::NEG_INFINITY],
    ];
    run_cases(2, 0x0202_0202_0202_0202, &fixed, |rng| {
        [rng.in_range(0.0, 60.0), rng.nonzero_f32(), rng.any_f32()]
    });
}

#[test]
fn config_03_hue_60_to_120() {
    let fixed = [
        [60.0, 1.0, 0.5],
        [below(120.0), -1.0, -0.0],
        [90.0, f32::NAN, f32::INFINITY],
    ];
    run_cases(3, 0x0303_0303_0303_0303, &fixed, |rng| {
        [rng.in_range(60.0, 120.0), rng.nonzero_f32(), rng.any_f32()]
    });
}

#[test]
fn config_04_negative_hue() {
    let fixed = [
        [f32::from_bits(0x8000_0001), 1.0, 0.5],
        [-60.0, -1.0, -0.0],
        [f32::NEG_INFINITY, f32::NAN, f32::INFINITY],
    ];
    run_cases(4, 0x0404_0404_0404_0404, &fixed, |rng| {
        let magnitude = rng.in_range(f32::from_bits(1), 10_000.0);
        [-magnitude, rng.nonzero_f32(), rng.any_f32()]
    });
}

#[test]
fn config_05_hue_180_to_240() {
    let fixed = [
        [180.0, 1.0, 0.5],
        [below(240.0), -1.0, -0.0],
        [210.0, f32::NAN, f32::NEG_INFINITY],
    ];
    run_cases(5, 0x0505_0505_0505_0505, &fixed, |rng| {
        [rng.in_range(180.0, 240.0), rng.nonzero_f32(), rng.any_f32()]
    });
}

#[test]
fn config_06_hue_240_to_300() {
    let fixed = [
        [240.0, 1.0, 0.5],
        [below(300.0), -1.0, -0.0],
        [270.0, f32::INFINITY, f32::NAN],
    ];
    run_cases(6, 0x0606_0606_0606_0606, &fixed, |rng| {
        [rng.in_range(240.0, 300.0), rng.nonzero_f32(), rng.any_f32()]
    });
}

#[test]
fn config_07_hue_300_to_360() {
    let fixed = [
        [300.0, 1.0, 0.5],
        [below(360.0), -1.0, -0.0],
        [330.0, f32::NAN, f32::INFINITY],
    ];
    run_cases(7, 0x0707_0707_0707_0707, &fixed, |rng| {
        [rng.in_range(300.0, 360.0), rng.nonzero_f32(), rng.any_f32()]
    });
}

#[test]
fn config_08_fallback_hue() {
    let fixed = [
        [120.0, 1.0, 0.5],
        [below(180.0), -1.0, -0.0],
        [360.0, f32::NAN, f32::INFINITY],
        [f32::INFINITY, 1.0, f32::NEG_INFINITY],
        [f32::from_bits(0x7fa1_2345), -1.0, f32::NAN],
    ];
    let mut branch = 0;
    run_cases(8, 0x0808_0808_0808_0808, &fixed, move |rng| {
        let h = match branch % 3 {
            0 => rng.in_range(120.0, 180.0),
            1 => rng.in_range(360.0, 10_000.0),
            _ => f32::from_bits(0x7f80_0001 | (rng.next_u32() & 0x007f_ffff)),
        };
        branch += 1;
        [h, rng.nonzero_f32(), rng.any_f32()]
    });
}

fn null_probe_status(implementation: &str, probe: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(probe)
        .arg("--nocapture")
        .env("HSL_NULL_PROBE_IMPLEMENTATION", implementation)
        .status()
        .unwrap()
}

#[cfg(unix)]
fn assert_same_termination(c_status: ExitStatus, rust_status: ExitStatus) {
    use std::os::unix::process::ExitStatusExt;

    assert!(
        !c_status.success(),
        "C unexpectedly accepted a null pointer"
    );
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "C and Rust terminated differently for a null pointer"
    );
}

#[test]
fn generic_boundary_null_src() {
    let c_status = null_probe_status("c", "probe_null_src");
    let rust_status = null_probe_status("rust", "probe_null_src");
    assert_same_termination(c_status, rust_status);
}

#[test]
fn generic_boundary_null_dest() {
    let c_status = null_probe_status("c", "probe_null_dest");
    let rust_status = null_probe_status("rust", "probe_null_dest");
    assert_same_termination(c_status, rust_status);
}

fn probe_function() -> Option<HslToRgb> {
    match std::env::var("HSL_NULL_PROBE_IMPLEMENTATION").as_deref() {
        Ok("c") => {
            let library = Box::leak(Box::new(load_library(&c_library_path())));
            Some(load_hsl_to_rgb(library))
        }
        Ok("rust") => {
            let library = Box::leak(Box::new(load_library(&rust_library_path())));
            Some(load_hsl_to_rgb(library))
        }
        _ => None,
    }
}

#[test]
fn probe_null_src() {
    let Some(function) = probe_function() else {
        return;
    };
    let mut output = [0.0; 3];
    unsafe { function(output.as_mut_ptr(), std::ptr::null()) };
}

#[test]
fn probe_null_dest() {
    let Some(function) = probe_function() else {
        return;
    };
    let input = [0.0, 0.0, 0.0];
    unsafe { function(std::ptr::null_mut(), input.as_ptr()) };
}
