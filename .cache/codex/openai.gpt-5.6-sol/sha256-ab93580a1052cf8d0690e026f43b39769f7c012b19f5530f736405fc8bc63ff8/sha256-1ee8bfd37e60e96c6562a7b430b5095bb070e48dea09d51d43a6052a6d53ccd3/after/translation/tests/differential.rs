use libloading::Library;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type RgbToHsv = unsafe extern "C" fn(*mut f32, *const f32);

const RANDOM_CASES: usize = 512;

struct Implementations {
    _c_library: Library,
    _rust_library: Library,
    c: RgbToHsv,
    rust: RgbToHsv,
}

impl Implementations {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        assert!(
            c_path.is_file(),
            "missing C shared object: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing Rust release shared object: {}; run cargo build --release first",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
            let c = *c_library
                .get::<RgbToHsv>(b"rgb_to_hsv\0")
                .expect("C library does not export rgb_to_hsv");
            let rust = *rust_library
                .get::<RgbToHsv>(b"rgb_to_hsv\0")
                .expect("Rust library does not export rgb_to_hsv");
            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c,
                rust,
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut candidates: Vec<_> = fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build_dir.display()))
        .map(|entry| entry.expect("invalid C build directory entry").path())
        .filter(|path| {
            path.is_file()
                && path.extension().and_then(|extension| extension.to_str())
                    == Some(&std::env::consts::DLL_EXTENSION)
        })
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared object in {}",
        build_dir.display()
    );
    candidates.remove(0)
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join(format!(
        "target/release/{}rgb_to_hsv_lib.{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_EXTENSION
    ))
}

fn bits(values: [f32; 3]) -> [u32; 3] {
    values.map(f32::to_bits)
}

fn call(function: RgbToHsv, input: [f32; 3]) -> [f32; 3] {
    let mut output = [f32::from_bits(0x7fa1_2345); 3];
    unsafe {
        function(output.as_mut_ptr(), input.as_ptr());
    }
    output
}

fn assert_case(implementations: &Implementations, input: [f32; 3]) {
    let c_output = call(implementations.c, input);
    let rust_output = call(implementations.rust, input);
    assert_eq!(
        bits(rust_output),
        bits(c_output),
        "input bits: {:08x?}; C: {:08x?}; Rust: {:08x?}",
        bits(input),
        bits(c_output),
        bits(rust_output)
    );
}

fn run_cases(cases: impl IntoIterator<Item = [f32; 3]>) {
    let implementations = Implementations::load();
    for input in cases {
        assert_case(&implementations, input);
    }
}

#[derive(Clone)]
struct Generator(u64);

impl Generator {
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

    fn bounded(&mut self, upper_exclusive: u32) -> u32 {
        self.next_u32() % upper_exclusive
    }

    fn small(&mut self) -> f32 {
        let magnitude = self.bounded(2_000_001) as i32 - 1_000_000;
        magnitude as f32 / 256.0
    }

    fn positive_gap(&mut self) -> f32 {
        (self.bounded(100_000) + 1) as f32 / 256.0
    }

    fn finite_bits(&mut self) -> f32 {
        loop {
            let candidate = f32::from_bits(self.next_u32());
            if candidate.is_finite() {
                return candidate;
            }
        }
    }

    fn nan(&mut self) -> f32 {
        let sign = self.next_u32() & 0x8000_0000;
        let payload = (self.next_u32() & 0x007f_ffff).max(1);
        f32::from_bits(sign | 0x7f80_0000 | payload)
    }
}

#[test]
fn config_01_finite_achromatic() {
    let mut generator = Generator::new(0x0101_a11c_e5ed);
    run_cases((0..RANDOM_CASES).map(|_| {
        let mut value = generator.small();
        if value == 0.0 {
            value = 1.0;
        }
        [value; 3]
    }));
}

#[test]
fn config_02_signed_zero_achromatic() {
    let mut generator = Generator::new(0x0202_5a1e_d000);
    run_cases((0..RANDOM_CASES).map(|_| {
        [0, 1, 2].map(|_| {
            if generator.next_u32() & 1 == 0 {
                0.0
            } else {
                -0.0
            }
        })
    }));
}

#[test]
fn config_03_non_achromatic_zero_maximum() {
    let mut generator = Generator::new(0x0303_0000_a11c);
    run_cases((0..RANDOM_CASES).map(|index| {
        let first = -generator.positive_gap();
        let second = first - generator.positive_gap();
        match index % 3 {
            0 => [0.0, first, second],
            1 => [first, -0.0, second],
            _ => [first, second, 0.0],
        }
    }));
}

#[test]
fn config_04_unique_red_maximum_without_hue_adjustment() {
    let mut generator = Generator::new(0x0404_0ed0_0001);
    run_cases((0..RANDOM_CASES).map(|_| {
        let base = generator.small();
        let b = base;
        let g = b + generator.positive_gap();
        let r = g + generator.positive_gap();
        [r, g, b]
    }));
}

#[test]
fn config_05_unique_red_maximum_with_hue_adjustment() {
    let mut generator = Generator::new(0x0505_0ed0_0002);
    run_cases((0..RANDOM_CASES).map(|_| {
        let base = generator.small();
        let g = base;
        let b = g + generator.positive_gap();
        let r = b + generator.positive_gap();
        [r, g, b]
    }));
}

#[test]
fn config_06_unique_green_maximum() {
    let mut generator = Generator::new(0x0606_90ee_0001);
    run_cases((0..RANDOM_CASES).map(|index| {
        let base = generator.small();
        let low = base;
        let middle = low + generator.positive_gap();
        let g = middle + generator.positive_gap();
        if index & 1 == 0 {
            [middle, g, low]
        } else {
            [low, g, middle]
        }
    }));
}

#[test]
fn config_07_unique_blue_maximum() {
    let mut generator = Generator::new(0x0707_b10e_0001);
    run_cases((0..RANDOM_CASES).map(|index| {
        let base = generator.small();
        let low = base;
        let middle = low + generator.positive_gap();
        let b = middle + generator.positive_gap();
        if index & 1 == 0 {
            [middle, low, b]
        } else {
            [low, middle, b]
        }
    }));
}

#[test]
fn config_08_red_green_maximum_tie() {
    let mut generator = Generator::new(0x0808_71e0_0001);
    run_cases((0..RANDOM_CASES).map(|_| {
        let b = generator.small();
        let maximum = b + generator.positive_gap();
        [maximum, maximum, b]
    }));
}

#[test]
fn config_09_red_blue_maximum_tie() {
    let mut generator = Generator::new(0x0909_71e0_0002);
    run_cases((0..RANDOM_CASES).map(|_| {
        let g = generator.small();
        let maximum = g + generator.positive_gap();
        [maximum, g, maximum]
    }));
}

#[test]
fn config_10_green_blue_maximum_tie() {
    let mut generator = Generator::new(0x1010_71e0_0003);
    run_cases((0..RANDOM_CASES).map(|_| {
        let r = generator.small();
        let maximum = r + generator.positive_gap();
        [r, maximum, maximum]
    }));
}

#[test]
fn config_11_subnormal_channels() {
    let mut generator = Generator::new(0x1111_5ab0_0a11);
    let mut cases = vec![
        [f32::from_bits(1), f32::from_bits(2), f32::from_bits(3)],
        [
            f32::from_bits(0x8000_0001),
            f32::from_bits(1),
            f32::from_bits(0x007f_ffff),
        ],
    ];
    cases.extend((0..RANDOM_CASES).map(|_| {
        [0, 1, 2].map(|_| {
            let sign = generator.next_u32() & 0x8000_0000;
            let magnitude = generator.bounded(0x0080_0000);
            f32::from_bits(sign | magnitude)
        })
    }));
    run_cases(cases);
}

#[test]
fn config_12_red_nan() {
    let mut generator = Generator::new(0x1212_0a0a_0001);
    run_cases((0..RANDOM_CASES).map(|_| {
        [
            generator.nan(),
            generator.finite_bits(),
            generator.finite_bits(),
        ]
    }));
}

#[test]
fn config_13_green_nan() {
    let mut generator = Generator::new(0x1313_0a0a_0002);
    run_cases((0..RANDOM_CASES).map(|_| {
        [
            generator.finite_bits(),
            generator.nan(),
            generator.finite_bits(),
        ]
    }));
}

#[test]
fn config_14_blue_nan() {
    let mut generator = Generator::new(0x1414_0a0a_0003);
    run_cases((0..RANDOM_CASES).map(|_| {
        [
            generator.finite_bits(),
            generator.finite_bits(),
            generator.nan(),
        ]
    }));
}

#[test]
fn config_15_positive_infinity_unique_maximum() {
    let mut generator = Generator::new(0x1515_1af1_0001);
    run_cases((0..RANDOM_CASES).map(|index| {
        let first = generator.small();
        let second = generator.small();
        match index % 3 {
            0 => [f32::INFINITY, first, second],
            1 => [first, f32::INFINITY, second],
            _ => [first, second, f32::INFINITY],
        }
    }));
}

#[test]
fn config_16_negative_infinity_unique_minimum() {
    let mut generator = Generator::new(0x1616_1af1_0002);
    run_cases((0..RANDOM_CASES).map(|index| {
        let first = generator.small();
        let second = generator.small();
        match index % 3 {
            0 => [f32::NEG_INFINITY, first, second],
            1 => [first, f32::NEG_INFINITY, second],
            _ => [first, second, f32::NEG_INFINITY],
        }
    }));
}

#[test]
fn config_17_multiple_infinities() {
    let mut generator = Generator::new(0x1717_1af1_0003);
    run_cases((0..RANDOM_CASES).map(|index| {
        let finite = generator.finite_bits();
        match index % 11 {
            0 => [f32::INFINITY, f32::INFINITY, f32::INFINITY],
            1 => [f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY],
            2 => [f32::INFINITY, f32::INFINITY, finite],
            3 => [f32::INFINITY, finite, f32::INFINITY],
            4 => [finite, f32::INFINITY, f32::INFINITY],
            5 => [f32::NEG_INFINITY, f32::NEG_INFINITY, finite],
            6 => [f32::NEG_INFINITY, finite, f32::NEG_INFINITY],
            7 => [finite, f32::NEG_INFINITY, f32::NEG_INFINITY],
            8 => [f32::INFINITY, f32::NEG_INFINITY, finite],
            9 => [f32::NEG_INFINITY, finite, f32::INFINITY],
            _ => [finite, f32::INFINITY, f32::NEG_INFINITY],
        }
    }));
}

#[test]
fn config_18_separate_arrays_random_bits() {
    let mut generator = Generator::new(0x1818_5e9a_a47e);
    run_cases(
        (0..(RANDOM_CASES * 4)).map(|_| [0, 1, 2].map(|_| f32::from_bits(generator.next_u32()))),
    );
}

fn call_in_place(function: RgbToHsv, input: [f32; 3]) -> [u32; 3] {
    let mut buffer = input;
    unsafe {
        function(buffer.as_mut_ptr(), buffer.as_ptr());
    }
    bits(buffer)
}

#[test]
fn config_19_exact_in_place() {
    let implementations = Implementations::load();
    let mut generator = Generator::new(0x1919_1ace_0001);
    for _ in 0..RANDOM_CASES {
        let input = [0, 1, 2].map(|_| f32::from_bits(generator.next_u32()));
        assert_eq!(
            call_in_place(implementations.rust, input),
            call_in_place(implementations.c, input),
            "in-place input bits: {:08x?}",
            bits(input)
        );
    }
}

fn call_partially_overlapping(
    function: RgbToHsv,
    initial: [f32; 5],
    destination_offset: usize,
    source_offset: usize,
) -> [u32; 5] {
    let mut buffer = initial;
    unsafe {
        function(
            buffer.as_mut_ptr().add(destination_offset),
            buffer.as_ptr().add(source_offset),
        );
    }
    buffer.map(f32::to_bits)
}

#[test]
fn config_20_partially_overlapping() {
    let implementations = Implementations::load();
    let mut generator = Generator::new(0x2020_0ae4_1a90);
    for index in 0..RANDOM_CASES {
        let initial = [0, 1, 2, 3, 4].map(|_| f32::from_bits(generator.next_u32()));
        let (destination_offset, source_offset) = if index & 1 == 0 { (1, 0) } else { (0, 1) };
        assert_eq!(
            call_partially_overlapping(
                implementations.rust,
                initial,
                destination_offset,
                source_offset,
            ),
            call_partially_overlapping(
                implementations.c,
                initial,
                destination_offset,
                source_offset,
            ),
            "overlap offsets dest={destination_offset}, src={source_offset}; input bits: {:08x?}",
            initial.map(f32::to_bits)
        );
    }
}

fn run_crash_probe(library: &Path, null_argument: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("test executable path is unavailable"))
        .args(["--exact", "ffi_crash_probe", "--ignored", "--nocapture"])
        .env("RGB_TO_HSV_PROBE_LIBRARY", library)
        .env("RGB_TO_HSV_PROBE_NULL", null_argument)
        .status()
        .expect("failed to run isolated null-pointer probe")
}

#[cfg(unix)]
fn assert_matching_sigsegv(null_argument: &str) {
    use std::os::unix::process::ExitStatusExt;

    let c_status = run_crash_probe(&c_library_path(), null_argument);
    let rust_status = run_crash_probe(&rust_library_path(), null_argument);
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "different process termination for null {null_argument}: C={c_status:?}, Rust={rust_status:?}"
    );
    assert_eq!(
        c_status.signal(),
        Some(11),
        "C null-{null_argument} probe did not receive SIGSEGV: {c_status:?}"
    );
}

#[test]
#[cfg(unix)]
fn error_01_null_destination() {
    assert_matching_sigsegv("dest");
}

#[test]
#[cfg(unix)]
fn error_02_null_source() {
    assert_matching_sigsegv("src");
}

#[test]
#[ignore = "invoked by the null-pointer differential tests in an isolated process"]
fn ffi_crash_probe() {
    let Ok(library_path) = std::env::var("RGB_TO_HSV_PROBE_LIBRARY") else {
        return;
    };
    let null_argument =
        std::env::var("RGB_TO_HSV_PROBE_NULL").expect("probe null argument is missing");

    unsafe {
        let library = Library::new(&library_path)
            .unwrap_or_else(|error| panic!("failed to load {library_path}: {error}"));
        let function = *library
            .get::<RgbToHsv>(b"rgb_to_hsv\0")
            .expect("probe library does not export rgb_to_hsv");
        match null_argument.as_str() {
            "dest" => {
                let input = [0.25, 0.5, 0.75];
                function(std::ptr::null_mut(), input.as_ptr());
            }
            "src" => {
                let mut output = [0.0; 3];
                function(output.as_mut_ptr(), std::ptr::null());
            }
            unexpected => panic!("unknown null argument: {unexpected}"),
        }
    }
}
