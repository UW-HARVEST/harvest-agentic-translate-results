use libloading::Library;
use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

type HslToRgb = unsafe extern "C" fn(*mut f32, *const f32);

const RANDOM_CASES: usize = 2_048;
const SEED: u64 = 0x6a09_e667_f3bc_c909;

struct Libraries {
    _c_library: Library,
    _rust_library: Library,
    c: HslToRgb,
    rust: HslToRgb,
}

impl Libraries {
    fn load() -> Self {
        let c_library =
            unsafe { Library::new(c_library_path()).expect("failed to load the C shared library") };
        let rust_library = unsafe {
            Library::new(rust_library_path()).expect("failed to load the Rust shared library")
        };
        let c = unsafe {
            *c_library
                .get::<HslToRgb>(b"hsl_to_rgb")
                .expect("C library does not export hsl_to_rgb")
        };
        let rust = unsafe {
            *rust_library
                .get::<HslToRgb>(b"hsl_to_rgb")
                .expect("Rust library does not export hsl_to_rgb")
        };

        Self {
            _c_library: c_library,
            _rust_library: rust_library,
            c,
            rust,
        }
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation must have a parent directory")
        .to_path_buf()
}

fn c_library_path() -> PathBuf {
    env::var_os("HSL_C_LIBRARY").map_or_else(
        || {
            let root = workspace_root();
            let project_name = root
                .file_name()
                .expect("workspace root must have a directory name")
                .to_string_lossy();
            root.join("c_src")
                .join("build")
                .join(format!("lib{project_name}.so"))
        },
        PathBuf::from,
    )
}

fn rust_library_path() -> PathBuf {
    env::var_os("HSL_RUST_LIBRARY").map_or_else(
        || {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("release")
                .join("libhsl_to_rgb_lib.so")
        },
        PathBuf::from,
    )
}

#[derive(Clone, Copy)]
enum HueClass {
    Negative,
    ZeroTo60,
    SixtyTo120,
    OneTwentyTo180,
    OneEightyTo240,
    TwoFortyTo300,
    ThreeHundredTo360,
    AtLeast360,
    Nan,
}

impl HueClass {
    fn contains(self, value: f32) -> bool {
        match self {
            Self::Negative => value < 0.0,
            Self::ZeroTo60 => value >= 0.0 && value < 60.0,
            Self::SixtyTo120 => value >= 60.0 && value < 120.0,
            Self::OneTwentyTo180 => value >= 120.0 && value < 180.0,
            Self::OneEightyTo240 => value >= 180.0 && value < 240.0,
            Self::TwoFortyTo300 => value >= 240.0 && value < 300.0,
            Self::ThreeHundredTo360 => value >= 300.0 && value < 360.0,
            Self::AtLeast360 => value >= 360.0,
            Self::Nan => value.is_nan(),
        }
    }

    fn boundaries(self) -> Vec<f32> {
        match self {
            Self::Negative => vec![f32::NEG_INFINITY, -f32::MAX, -60.0, -f32::MIN_POSITIVE],
            Self::ZeroTo60 => vec![0.0, -0.0, f32::MIN_POSITIVE, 59.999_996],
            Self::SixtyTo120 => vec![60.0, 60.000_004, 119.999_99],
            Self::OneTwentyTo180 => vec![120.0, 120.000_01, 179.999_98],
            Self::OneEightyTo240 => vec![180.0, 180.000_02, 239.999_98],
            Self::TwoFortyTo300 => vec![240.0, 240.000_02, 299.999_97],
            Self::ThreeHundredTo360 => vec![300.0, 300.000_03, 359.999_97],
            Self::AtLeast360 => vec![360.0, 360.000_03, f32::MAX, f32::INFINITY],
            Self::Nan => vec![
                f32::from_bits(0x7fc0_0000),
                f32::from_bits(0x7f80_0001),
                f32::from_bits(0xffc0_1234),
            ],
        }
    }
}

struct Rng(u64);

impl Rng {
    fn new(row: u64) -> Self {
        Self(SEED ^ row.wrapping_mul(0x9e37_79b9_7f4a_7c15))
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        (value >> 32) as u32 ^ value as u32
    }

    fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    fn nonzero_f32(&mut self) -> f32 {
        loop {
            let value = self.any_f32();
            if value != 0.0 {
                return value;
            }
        }
    }

    fn hue(&mut self, class: HueClass) -> f32 {
        if matches!(class, HueClass::Nan) {
            let sign = self.next_u32() & 0x8000_0000;
            let payload = (self.next_u32() & 0x007f_ffff).max(1);
            return f32::from_bits(sign | 0x7f80_0000 | payload);
        }

        loop {
            let value = self.any_f32();
            if class.contains(value) {
                return value;
            }
        }
    }
}

fn call(function: HslToRgb, input: [f32; 3]) -> [u32; 3] {
    let mut output = [
        f32::from_bits(0xdead_beef),
        f32::from_bits(0xdead_beef),
        f32::from_bits(0xdead_beef),
    ];
    unsafe { function(output.as_mut_ptr(), input.as_ptr()) };
    output.map(f32::to_bits)
}

fn assert_differential(libraries: &Libraries, row: usize, iteration: usize, input: [f32; 3]) {
    let c_output = call(libraries.c, input);
    let rust_output = call(libraries.rust, input);
    assert_eq!(
        rust_output,
        c_output,
        "CONFIGS.md row {row}, iteration {iteration}, input bits {:08x?}",
        input.map(f32::to_bits)
    );
}

fn run_zero_saturation_row(row: usize, saturation: f32) {
    let libraries = Libraries::load();
    let mut rng = Rng::new(row as u64);
    let fixed = [
        [0.0, saturation, 0.0],
        [f32::NAN, saturation, f32::from_bits(0x7f80_0001)],
        [f32::INFINITY, saturation, f32::NEG_INFINITY],
    ];

    for (iteration, input) in fixed.into_iter().enumerate() {
        assert_differential(&libraries, row, iteration, input);
    }
    for iteration in 0..RANDOM_CASES {
        let input = [rng.any_f32(), saturation, rng.any_f32()];
        assert_differential(&libraries, row, fixed.len() + iteration, input);
    }
}

fn run_hue_row(row: usize, class: HueClass) {
    let libraries = Libraries::load();
    let mut rng = Rng::new(row as u64);
    let boundaries = class.boundaries();

    for (iteration, &hue) in boundaries.iter().enumerate() {
        let input = [hue, rng.nonzero_f32(), rng.any_f32()];
        assert_differential(&libraries, row, iteration, input);
    }
    for iteration in 0..RANDOM_CASES {
        let input = [rng.hue(class), rng.nonzero_f32(), rng.any_f32()];
        assert_differential(&libraries, row, boundaries.len() + iteration, input);
    }
}

#[test]
fn config_01_positive_zero_saturation() {
    run_zero_saturation_row(1, 0.0);
}

#[test]
fn config_02_negative_zero_saturation() {
    run_zero_saturation_row(2, -0.0);
}

#[test]
fn config_03_negative_hue() {
    run_hue_row(3, HueClass::Negative);
}

#[test]
fn config_04_zero_to_60_hue() {
    run_hue_row(4, HueClass::ZeroTo60);
}

#[test]
fn config_05_60_to_120_hue() {
    run_hue_row(5, HueClass::SixtyTo120);
}

#[test]
fn config_06_120_to_180_hue() {
    run_hue_row(6, HueClass::OneTwentyTo180);
}

#[test]
fn config_07_180_to_240_hue() {
    run_hue_row(7, HueClass::OneEightyTo240);
}

#[test]
fn config_08_240_to_300_hue() {
    run_hue_row(8, HueClass::TwoFortyTo300);
}

#[test]
fn config_09_300_to_360_hue() {
    run_hue_row(9, HueClass::ThreeHundredTo360);
}

#[test]
fn config_10_at_least_360_hue() {
    run_hue_row(10, HueClass::AtLeast360);
}

#[test]
fn config_11_nan_hue() {
    run_hue_row(11, HueClass::Nan);
}

#[test]
fn null_pointer_child() {
    let Ok(case) = env::var("HSL_NULL_CASE") else {
        return;
    };
    let path = env::var_os("HSL_NULL_LIBRARY").expect("HSL_NULL_LIBRARY must be set");
    let library = unsafe { Library::new(path).expect("failed to load child shared library") };
    let function = unsafe {
        *library
            .get::<HslToRgb>(b"hsl_to_rgb")
            .expect("child library does not export hsl_to_rgb")
    };

    match case.as_str() {
        "dest" => {
            let input = [0.0, 0.0, 0.5];
            unsafe { function(std::ptr::null_mut(), input.as_ptr()) };
        }
        "src" => {
            let mut output = [0.0; 3];
            unsafe { function(output.as_mut_ptr(), std::ptr::null()) };
        }
        _ => panic!("unknown HSL_NULL_CASE value"),
    }
}

fn run_null_child(library: PathBuf, case: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("failed to find current test executable"))
        .args(["--exact", "null_pointer_child", "--nocapture"])
        .env("HSL_NULL_LIBRARY", library)
        .env("HSL_NULL_CASE", case)
        .status()
        .expect("failed to run null-pointer child")
}

fn assert_matching_null_termination(case: &str) {
    let c_status = run_null_child(c_library_path(), case);
    let rust_status = run_null_child(rust_library_path(), case);
    assert!(
        !c_status.success(),
        "C null-{case} child unexpectedly returned"
    );
    assert!(
        !rust_status.success(),
        "Rust null-{case} child unexpectedly returned"
    );

    #[cfg(unix)]
    {
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "C and Rust null-{case} calls terminated differently"
        );
        assert_eq!(
            c_status.signal(),
            Some(11),
            "C null-{case} call did not terminate with SIGSEGV"
        );
    }
}

#[test]
fn error_01_null_destination() {
    assert_matching_null_termination("dest");
}

#[test]
fn error_02_null_source() {
    assert_matching_null_termination("src");
}
