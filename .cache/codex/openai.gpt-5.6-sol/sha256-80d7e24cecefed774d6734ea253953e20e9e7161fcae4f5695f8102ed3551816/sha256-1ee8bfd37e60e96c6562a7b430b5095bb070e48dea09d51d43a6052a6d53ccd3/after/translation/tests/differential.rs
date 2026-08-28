use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use libloading::Library;

type HsvToRgb = unsafe extern "C" fn(*mut f32, *const f32);

struct Api {
    hsv_to_rgb: HsvToRgb,
    _library: Library,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let hsv_to_rgb = unsafe {
            *library
                .get::<HsvToRgb>(b"hsv_to_rgb\0")
                .unwrap_or_else(|error| {
                    panic!("failed to load hsv_to_rgb from {}: {error}", path.display())
                })
        };
        Self {
            hsv_to_rgb,
            _library: library,
        }
    }

    fn convert(&self, input: [f32; 3], in_place: bool) -> [u32; 3] {
        let output = if in_place {
            let mut buffer = input;
            unsafe { (self.hsv_to_rgb)(buffer.as_mut_ptr(), buffer.as_ptr()) };
            buffer
        } else {
            let mut output = [
                f32::from_bits(0x7fc0_0001),
                f32::from_bits(0x7fc0_0002),
                f32::from_bits(0x7fc0_0003),
            ];
            unsafe { (self.hsv_to_rgb)(output.as_mut_ptr(), input.as_ptr()) };
            output
        };
        output.map(f32::to_bits)
    }
}

struct Libraries {
    c: Api,
    rust: Api,
}

impl Libraries {
    fn load() -> Self {
        unsafe {
            Self {
                c: Api::load(&c_library_path()),
                rust: Api::load(&rust_library_path()),
            }
        }
    }

    fn assert_same(&self, input: [f32; 3]) {
        for in_place in [false, true] {
            let c = self.c.convert(input, in_place);
            let rust = self.rust.convert(input, in_place);
            assert_eq!(
                c,
                rust,
                "input bits={:08x?}, in_place={in_place}",
                input.map(f32::to_bits)
            );
        }
    }
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    crate_dir().join("../c_src/build/libharvest-work-GPFm3T.so")
}

fn rust_library_path() -> PathBuf {
    crate_dir().join("target/release/libhsv_to_rgb_lib.so")
}

struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn unit(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) / ((1_u32 << 24) as f32)
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }
}

fn randomized_sector(sector: i32, seed: u64) {
    let libraries = Libraries::load();
    let mut rng = SplitMix64::new(seed);

    for _ in 0..4096 {
        let h = sector as f32 * 60.0 + rng.range(0.125, 59.875);
        let mut s = rng.range(-4.0, 4.0);
        if s == 0.0 {
            s = f32::MIN_POSITIVE;
        }
        let v = rng.range(-8.0, 8.0);
        libraries.assert_same([h, s, v]);
    }

    let low = sector as f32 * 60.0;
    let high = (sector + 1) as f32 * 60.0;
    for h in [
        low,
        f32::from_bits(low.to_bits() + 1),
        f32::from_bits(high.to_bits() - 1),
    ] {
        for s in [
            f32::MIN_POSITIVE,
            f32::from_bits(1),
            0.5,
            1.0,
            f32::from_bits(1.0_f32.to_bits() + 1),
            2.0,
            -1.0,
        ] {
            for v in [
                -2.0,
                -0.0,
                0.0,
                f32::from_bits(1),
                0.5,
                1.0,
                f32::from_bits(1.0_f32.to_bits() + 1),
                2.0,
            ] {
                libraries.assert_same([h, s, v]);
            }
        }
    }

    let midpoint = low + 30.0;
    for input in [
        [midpoint, f32::INFINITY, 1.0],
        [midpoint, f32::NEG_INFINITY, 1.0],
        [midpoint, f32::from_bits(0x7fc0_1234), 1.0],
        [midpoint, 0.5, f32::INFINITY],
        [midpoint, 0.5, f32::NEG_INFINITY],
        [midpoint, 0.5, f32::from_bits(0x7fc0_5678)],
    ] {
        libraries.assert_same(input);
    }
}

#[test]
fn config_01_zero_saturation() {
    let libraries = Libraries::load();
    let mut rng = SplitMix64::new(0xa11c_e001);

    for index in 0..4096 {
        let h = f32::from_bits(rng.next_u64() as u32);
        let v = f32::from_bits(rng.next_u64() as u32);
        let s = if index % 2 == 0 { 0.0 } else { -0.0 };
        libraries.assert_same([h, s, v]);
    }
}

#[test]
fn config_02_sector_0() {
    randomized_sector(0, 0xa11c_e002);
}

#[test]
fn config_03_sector_1() {
    randomized_sector(1, 0xa11c_e003);
}

#[test]
fn config_04_sector_2() {
    randomized_sector(2, 0xa11c_e004);
}

#[test]
fn config_05_sector_3() {
    randomized_sector(3, 0xa11c_e005);
}

#[test]
fn config_06_sector_4() {
    randomized_sector(4, 0xa11c_e006);
}

#[test]
fn config_07_default_sector() {
    let libraries = Libraries::load();
    let mut rng = SplitMix64::new(0xa11c_e007);
    let sectors = [-100, -2, -1, 5, 6, 100];

    for index in 0..8192 {
        let sector = sectors[index % sectors.len()];
        let h = sector as f32 * 60.0 + rng.range(0.125, 59.875);
        let mut s = rng.range(-4.0, 4.0);
        if s == 0.0 {
            s = f32::MIN_POSITIVE;
        }
        libraries.assert_same([h, s, rng.range(-8.0, 8.0)]);
    }

    let special_inputs = [
        [-60.0, 1.0, 1.0],
        [300.0, 1.0, 1.0],
        [360.0, 1.0, 1.0],
        [f32::MAX, 0.5, 1.0],
        [f32::MIN, 0.5, 1.0],
        [f32::INFINITY, 0.5, 1.0],
        [f32::NEG_INFINITY, 0.5, 1.0],
        [f32::from_bits(0x7fc0_1234), 0.5, 1.0],
    ];
    for input in special_inputs {
        libraries.assert_same(input);
    }
}

#[cfg(unix)]
fn signal(status: ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[test]
fn generic_null_pointer_behavior() {
    for pointer in ["dest", "src"] {
        let c = run_null_child(&c_library_path(), pointer);
        let rust = run_null_child(&rust_library_path(), pointer);
        assert!(!c.success(), "C unexpectedly accepted null {pointer}");
        assert!(!rust.success(), "Rust unexpectedly accepted null {pointer}");
        #[cfg(unix)]
        assert_eq!(
            signal(c),
            signal(rust),
            "different process signals for null {pointer}: C={c:?}, Rust={rust:?}"
        );
    }
}

fn run_null_child(library: &Path, pointer: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "null_pointer_child", "--nocapture"])
        .env("HSV_NULL_CHILD_LIBRARY", library)
        .env("HSV_NULL_CHILD_POINTER", pointer)
        .status()
        .unwrap_or_else(|error| panic!("failed to run null-pointer child: {error}"))
}

#[test]
fn null_pointer_child() {
    let Some(library) = std::env::var_os("HSV_NULL_CHILD_LIBRARY") else {
        return;
    };
    let pointer = std::env::var("HSV_NULL_CHILD_POINTER").expect("child pointer selection");
    let api = unsafe { Api::load(Path::new(&library)) };
    let mut output = [0.0_f32; 3];
    let input = [0.0_f32, 0.5, 1.0];

    unsafe {
        match pointer.as_str() {
            "dest" => (api.hsv_to_rgb)(std::ptr::null_mut(), input.as_ptr()),
            "src" => (api.hsv_to_rgb)(output.as_mut_ptr(), std::ptr::null()),
            _ => panic!("unknown child pointer selection: {pointer}"),
        }
    }
}
