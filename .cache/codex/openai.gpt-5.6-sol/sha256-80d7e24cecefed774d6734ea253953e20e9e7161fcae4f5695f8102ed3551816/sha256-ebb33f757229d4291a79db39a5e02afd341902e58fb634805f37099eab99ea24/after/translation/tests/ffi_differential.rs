use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type HsvToRgb = unsafe extern "C" fn(*mut f32, *const f32);

const CASES_PER_ROW: usize = 2_048;

struct LoadedLibrary {
    _library: Library,
    function: HsvToRgb,
}

impl LoadedLibrary {
    unsafe fn open(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let function = {
            let symbol: Symbol<HsvToRgb> =
                unsafe { library.get(b"hsv_to_rgb\0") }.unwrap_or_else(|error| {
                    panic!("failed to load hsv_to_rgb from {}: {error}", path.display())
                });
            *symbol
        };
        Self {
            _library: library,
            function,
        }
    }

    fn call(&self, src: [f32; 3]) -> [f32; 3] {
        let mut dest = [
            f32::from_bits(0x7fc0_1234),
            f32::from_bits(0x7fc0_5678),
            f32::from_bits(0x7fc0_9abc),
        ];
        unsafe { (self.function)(dest.as_mut_ptr(), src.as_ptr()) };
        dest
    }

    fn call_in_place(&self, src: [f32; 3]) -> [f32; 3] {
        let mut buffer = src;
        unsafe { (self.function)(buffer.as_mut_ptr(), buffer.as_ptr()) };
        buffer
    }
}

struct Libraries {
    c: LoadedLibrary,
    rust: LoadedLibrary,
}

impl Libraries {
    fn load() -> Self {
        unsafe {
            Self {
                c: LoadedLibrary::open(&c_library_path()),
                rust: LoadedLibrary::open(&rust_library_path()),
            }
        }
    }

    fn assert_equal(&self, src: [f32; 3]) {
        let c = self.c.call(src);
        let rust = self.rust.call(src);
        assert_float_bytes_equal(src, c, rust, "separate buffers");

        let c_in_place = self.c.call_in_place(src);
        let rust_in_place = self.rust.call_in_place(src);
        assert_float_bytes_equal(src, c_in_place, rust_in_place, "in-place");
        assert_float_bytes_equal(src, c, c_in_place, "C alias behavior");
        assert_float_bytes_equal(src, rust, rust_in_place, "Rust alias behavior");
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    fn arbitrary_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    let build_dir = manifest_dir().join("../c_src/build");
    let mut candidates: Vec<_> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", build_dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "so"))
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared library in {}, found {candidates:?}",
        build_dir.display()
    );
    candidates.remove(0)
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libhsv_to_rgb_lib.so")
}

fn float_bytes(values: [f32; 3]) -> [u8; 12] {
    let mut bytes = [0_u8; 12];
    for (index, value) in values.into_iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

fn assert_float_bytes_equal(src: [f32; 3], left: [f32; 3], right: [f32; 3], context: &str) {
    assert_eq!(
        float_bytes(left),
        float_bytes(right),
        "{context} mismatch for input bits [{:#010x}, {:#010x}, {:#010x}]\nleft:  [{:#010x}, {:#010x}, {:#010x}]\nright: [{:#010x}, {:#010x}, {:#010x}]",
        src[0].to_bits(),
        src[1].to_bits(),
        src[2].to_bits(),
        left[0].to_bits(),
        left[1].to_bits(),
        left[2].to_bits(),
        right[0].to_bits(),
        right[1].to_bits(),
        right[2].to_bits(),
    );
}

fn exercise_sector(sector: i32, seed: u64) {
    let libraries = Libraries::load();
    let mut rng = Rng::new(seed);
    let start = sector as f32 * 60.0;

    for boundary in [
        start,
        f32::from_bits(start.to_bits() + 1),
        (start + 60.0).next_down(),
    ] {
        for saturation in [
            f32::MIN_POSITIVE,
            0.25,
            1.0,
            -0.5,
            2.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
        ] {
            for value in [
                0.0,
                -0.0,
                f32::MIN_POSITIVE,
                1.0,
                -1.0,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::from_bits(0x7fc0_4321),
            ] {
                libraries.assert_equal([boundary, saturation, value]);
            }
        }
    }

    for _ in 0..CASES_PER_ROW {
        let hue = rng.range(start, start + 60.0);
        let mut saturation = rng.range(-4.0, 4.0);
        if saturation == 0.0 {
            saturation = f32::MIN_POSITIVE;
        }
        let value = rng.range(-1.0e6, 1.0e6);
        libraries.assert_equal([hue, saturation, value]);
    }
}

#[test]
fn config_1_zero_saturation() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x7865_726f_5f73_6174);

    for saturation in [0.0, -0.0] {
        for value in [
            0.0,
            -0.0,
            f32::MIN_POSITIVE,
            f32::MAX,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(0x7fc0_9876),
        ] {
            libraries.assert_equal([f32::from_bits(0x7fc0_1234), saturation, value]);
        }
    }

    for index in 0..CASES_PER_ROW {
        let saturation = if index % 2 == 0 { 0.0 } else { -0.0 };
        libraries.assert_equal([rng.arbitrary_f32(), saturation, rng.arbitrary_f32()]);
    }
}

#[test]
fn config_2_sector_0() {
    exercise_sector(0, 0x7365_6374_6f72_5f30);
}

#[test]
fn config_3_sector_1() {
    exercise_sector(1, 0x7365_6374_6f72_5f31);
}

#[test]
fn config_4_sector_2() {
    exercise_sector(2, 0x7365_6374_6f72_5f32);
}

#[test]
fn config_5_sector_3() {
    exercise_sector(3, 0x7365_6374_6f72_5f33);
}

#[test]
fn config_6_sector_4() {
    exercise_sector(4, 0x7365_6374_6f72_5f34);
}

#[test]
fn config_7_default_sector() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x6465_6661_756c_745f);

    let special_hues = [
        300.0,
        360.0_f32.next_down(),
        360.0,
        420.0,
        -0.0,
        -f32::MIN_POSITIVE,
        -60.0,
        f32::MAX,
        -f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0xffc0_5678),
    ];
    let special_saturations = [
        f32::MIN_POSITIVE,
        0.25,
        1.0,
        -1.0,
        2.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_4321),
    ];
    let special_values = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_8765),
    ];

    for hue in special_hues {
        for saturation in special_saturations {
            for value in special_values {
                libraries.assert_equal([hue, saturation, value]);
            }
        }
    }

    for index in 0..CASES_PER_ROW {
        let hue = match index % 3 {
            0 => rng.range(300.0, 360.0),
            1 => rng.range(-1.0e6, -f32::MIN_POSITIVE),
            _ => rng.range(360.0, 1.0e6),
        };
        let mut saturation = rng.range(-4.0, 4.0);
        if saturation == 0.0 {
            saturation = f32::MIN_POSITIVE;
        }
        libraries.assert_equal([hue, saturation, rng.range(-1.0e6, 1.0e6)]);
    }
}

fn run_null_probe(library: &Path, probe: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("current test executable"))
        .arg("--exact")
        .arg("null_pointer_child")
        .arg("--nocapture")
        .env("HSV_NULL_PROBE_LIBRARY", library)
        .env("HSV_NULL_PROBE_KIND", probe)
        .status()
        .expect("run null-pointer child")
}

#[cfg(unix)]
fn assert_same_abnormal_termination(c: ExitStatus, rust: ExitStatus, probe: &str) {
    use std::os::unix::process::ExitStatusExt;

    assert!(!c.success(), "C {probe} probe unexpectedly succeeded");
    assert!(!rust.success(), "Rust {probe} probe unexpectedly succeeded");
    assert_eq!(
        c.signal(),
        rust.signal(),
        "{probe} probe terminated with different signals: C={c:?}, Rust={rust:?}"
    );
    assert_eq!(
        c.code(),
        rust.code(),
        "{probe} probe terminated with different exit codes: C={c:?}, Rust={rust:?}"
    );
}

#[test]
fn generic_null_src_boundary() {
    let c = run_null_probe(&c_library_path(), "src");
    let rust = run_null_probe(&rust_library_path(), "src");
    assert_same_abnormal_termination(c, rust, "null src");
}

#[test]
fn generic_null_dest_boundary() {
    let c = run_null_probe(&c_library_path(), "dest");
    let rust = run_null_probe(&rust_library_path(), "dest");
    assert_same_abnormal_termination(c, rust, "null dest");
}

#[test]
fn null_pointer_child() {
    let Some(library_path) = std::env::var_os("HSV_NULL_PROBE_LIBRARY") else {
        return;
    };
    let probe = std::env::var("HSV_NULL_PROBE_KIND").expect("probe kind");
    let library = unsafe { LoadedLibrary::open(Path::new(&library_path)) };
    let mut dest = [0.0_f32; 3];
    let src = [0.0_f32, 0.0, 0.0];

    unsafe {
        match probe.as_str() {
            "src" => (library.function)(dest.as_mut_ptr(), std::ptr::null()),
            "dest" => (library.function)(std::ptr::null_mut(), src.as_ptr()),
            _ => panic!("unknown probe {probe}"),
        }
    }
}
