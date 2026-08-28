use libloading::Library;
use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::process::Command;

const STRUCT_SIZE: usize = 28;
const RANDOM_CASES_PER_CONFIG: usize = 256;
const FIXED_SEED: u64 = 0xd1ff_e2e0_5eed_cafe;

#[repr(C, align(4))]
#[derive(Clone, Debug, Eq, PartialEq)]
struct RawTflac {
    bytes: [u8; STRUCT_SIZE],
}

type ValidateFn = unsafe extern "C" fn(*mut RawTflac) -> c_int;
type SizeMemoryFn = unsafe extern "C" fn(u32) -> u32;

struct Api {
    _library: Library,
    validate: ValidateFn,
    size_memory: SizeMemoryFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let validate = unsafe {
            *library
                .get::<ValidateFn>(b"flac_validate\0")
                .unwrap_or_else(|error| panic!("missing flac_validate: {error}"))
        };
        let size_memory = unsafe {
            *library
                .get::<SizeMemoryFn>(b"tflac_size_memory\0")
                .unwrap_or_else(|error| panic!("missing tflac_size_memory: {error}"))
        };
        Self {
            _library: library,
            validate,
            size_memory,
        }
    }
}

struct Pair {
    c: Api,
    rust: Api,
}

impl Pair {
    fn load() -> Self {
        unsafe {
            Self {
                c: Api::load(&c_library_path()),
                rust: Api::load(&rust_library_path()),
            }
        }
    }

    fn validate(&self, input: &RawTflac, context: &str) {
        let mut c_value = input.clone();
        let mut rust_value = input.clone();
        let c_result = unsafe { (self.c.validate)(&mut c_value) };
        let rust_result = unsafe { (self.rust.validate)(&mut rust_value) };

        assert_eq!(
            c_result, 0,
            "{context}: generated valid case was rejected by C for input {input:?}"
        );
        assert_eq!(
            rust_result, c_result,
            "{context}: return value differs for input {input:?}"
        );
        assert_eq!(
            rust_value, c_value,
            "{context}: output bytes differ for input {input:?}"
        );
    }
}

fn c_library_path() -> PathBuf {
    let build_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build");
    let mut candidates: Vec<_> = std::fs::read_dir(&build_dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", build_dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|value| value.to_str()) == Some("so")
                && path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|name| name.starts_with("libharvest-work-"))
        })
        .collect();
    candidates.sort();
    assert_eq!(
        candidates.len(),
        1,
        "expected exactly one C shared library in {}",
        build_dir.display()
    );
    candidates.remove(0)
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libflac_validate_lib.so")
}

#[derive(Clone, Copy, Debug)]
enum ModeClass {
    Independent,
    PreservedNonzero,
    NormalizedChannels,
    NormalizedDepth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RiceClass {
    DefaultLowDepth,
    DefaultHighDepth,
    Explicit,
}

#[derive(Clone, Copy, Debug)]
enum PartitionClass {
    EqualLimits,
    NoAdvance,
    AdvanceToMax,
    AdvancePartway,
}

#[derive(Clone, Copy, Debug)]
struct Config {
    row: usize,
    mode: ModeClass,
    rice: RiceClass,
    partition: PartitionClass,
}

#[derive(Clone, Copy)]
struct XorShift64(u64);

impl XorShift64 {
    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn u32_inclusive(&mut self, min: u32, max: u32) -> u32 {
        min + (self.next_u64() % u64::from(max - min + 1)) as u32
    }

    fn u8_inclusive(&mut self, min: u8, max: u8) -> u8 {
        self.u32_inclusive(u32::from(min), u32::from(max)) as u8
    }
}

fn put_u32(bytes: &mut [u8; STRUCT_SIZE], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
}

fn valid_input(config: Config, iteration: usize, rng: &mut XorShift64) -> RawTflac {
    let (min_partition_order, max_partition_order, blocksize) =
        partition_values(config.partition, rng);

    let bitdepth = match (config.mode, config.rice) {
        (ModeClass::NormalizedDepth, _) => 32,
        (ModeClass::PreservedNonzero, RiceClass::DefaultHighDepth) => rng.u32_inclusive(17, 31),
        (_, RiceClass::DefaultLowDepth) => rng.u32_inclusive(1, 16),
        (_, RiceClass::DefaultHighDepth) => rng.u32_inclusive(17, 32),
        (ModeClass::PreservedNonzero, RiceClass::Explicit) => rng.u32_inclusive(1, 31),
        (_, RiceClass::Explicit) => rng.u32_inclusive(1, 32),
    };

    let channels = match config.mode {
        ModeClass::PreservedNonzero | ModeClass::NormalizedDepth => 2,
        ModeClass::NormalizedChannels => {
            let value = rng.u32_inclusive(1, 7);
            if value >= 2 { value + 1 } else { value }
        }
        ModeClass::Independent => rng.u32_inclusive(1, 8),
    };

    let channel_mode = match config.mode {
        ModeClass::Independent => 0,
        _ => {
            const VALUES: [u8; 7] = [1, 2, 3, 4, 127, 254, 255];
            if iteration < VALUES.len() {
                VALUES[iteration]
            } else {
                rng.u8_inclusive(1, 255)
            }
        }
    };

    let max_rice_value = match config.rice {
        RiceClass::DefaultLowDepth | RiceClass::DefaultHighDepth => 0,
        RiceClass::Explicit => rng.u8_inclusive(1, 30),
    };

    let samplerate = match iteration {
        0 => 1,
        1 => 655_350,
        _ => rng.u32_inclusive(1, 655_350),
    };

    let mut bytes = [0u8; STRUCT_SIZE];
    put_u32(&mut bytes, 0, blocksize);
    put_u32(&mut bytes, 4, samplerate);
    put_u32(&mut bytes, 8, channels);
    put_u32(&mut bytes, 12, bitdepth);
    bytes[16] = channel_mode;
    bytes[17] = max_rice_value;
    bytes[18] = min_partition_order;
    bytes[19] = max_partition_order;
    bytes[20] = rng.u8_inclusive(0, 255);
    bytes[21] = rng.u8_inclusive(0, 255);
    bytes[22] = rng.u8_inclusive(0, 255);
    bytes[23] = rng.u8_inclusive(0, 255);
    put_u32(&mut bytes, 24, rng.next_u64() as u32);
    RawTflac { bytes }
}

fn partition_values(class: PartitionClass, rng: &mut XorShift64) -> (u8, u8, u32) {
    match class {
        PartitionClass::EqualLimits => {
            let order = rng.u8_inclusive(0, 15);
            (order, order, rng.u32_inclusive(16, 65_535))
        }
        PartitionClass::NoAdvance => {
            let min = rng.u8_inclusive(0, 14);
            let max = rng.u8_inclusive(min + 1, 15);
            let blocksize = rng.u32_inclusive(8, 32_767) * 2 + 1;
            (min, max, blocksize)
        }
        PartitionClass::AdvanceToMax => {
            let max = rng.u8_inclusive(1, 15);
            let min = rng.u8_inclusive(0, max - 1);
            let divisor = 1_u32 << max;
            let multiplier_min = 16_u32.div_ceil(divisor);
            let multiplier_max = 65_535 / divisor;
            let multiplier = rng.u32_inclusive(multiplier_min.max(1), multiplier_max);
            (min, max, divisor * multiplier)
        }
        PartitionClass::AdvancePartway => {
            let min = rng.u8_inclusive(0, 13);
            let max = rng.u8_inclusive(min + 2, 15);
            let stop = rng.u8_inclusive(min + 1, max - 1);
            let divisor = 1_u32 << stop;
            let mut multiplier_min = 16_u32.div_ceil(divisor).max(1);
            if multiplier_min % 2 == 0 {
                multiplier_min += 1;
            }
            let multiplier_max = 65_535 / divisor;
            let odd_count = ((multiplier_max - multiplier_min) / 2) + 1;
            let multiplier = multiplier_min + 2 * rng.u32_inclusive(0, odd_count - 1);
            (min, max, divisor * multiplier)
        }
    }
}

fn configurations() -> Vec<Config> {
    let modes = [
        ModeClass::Independent,
        ModeClass::PreservedNonzero,
        ModeClass::NormalizedChannels,
        ModeClass::NormalizedDepth,
    ];
    let rices = [
        RiceClass::DefaultLowDepth,
        RiceClass::DefaultHighDepth,
        RiceClass::Explicit,
    ];
    let partitions = [
        PartitionClass::EqualLimits,
        PartitionClass::NoAdvance,
        PartitionClass::AdvanceToMax,
        PartitionClass::AdvancePartway,
    ];
    let mut row = 2;
    let mut result = Vec::new();

    for mode in modes {
        for rice in rices {
            if matches!(mode, ModeClass::NormalizedDepth) && rice == RiceClass::DefaultLowDepth {
                continue;
            }
            for partition in partitions {
                result.push(Config {
                    row,
                    mode,
                    rice,
                    partition,
                });
                row += 1;
            }
        }
    }

    assert_eq!(result.len(), 44);
    assert_eq!(row, 46);
    result
}

fn baseline() -> RawTflac {
    let mut bytes = [0x5a; STRUCT_SIZE];
    put_u32(&mut bytes, 0, 4096);
    put_u32(&mut bytes, 4, 48_000);
    put_u32(&mut bytes, 8, 2);
    put_u32(&mut bytes, 12, 24);
    bytes[16] = 3;
    bytes[17] = 30;
    bytes[18] = 0;
    bytes[19] = 8;
    bytes[20] = 0xa5;
    put_u32(&mut bytes, 24, 0xdead_beef);
    RawTflac { bytes }
}

fn with_u32(mut value: RawTflac, offset: usize, field: u32) -> RawTflac {
    put_u32(&mut value.bytes, offset, field);
    value
}

fn with_u8(mut value: RawTflac, offset: usize, field: u8) -> RawTflac {
    value.bytes[offset] = field;
    value
}

#[test]
fn configuration_row_1_size_memory_matches() {
    let pair = Pair::load();
    let boundaries = [
        0,
        1,
        3,
        4,
        15,
        16,
        65_535,
        0x3fff_fffc,
        0x3fff_ffff,
        0x4000_0000,
        u32::MAX,
    ];
    for input in boundaries {
        let c_result = unsafe { (pair.c.size_memory)(input) };
        let rust_result = unsafe { (pair.rust.size_memory)(input) };
        assert_eq!(rust_result, c_result, "CONFIGS.md row 1, input {input}");
    }

    let mut rng = XorShift64(FIXED_SEED);
    for _ in 0..100_000 {
        let input = rng.next_u64() as u32;
        let c_result = unsafe { (pair.c.size_memory)(input) };
        let rust_result = unsafe { (pair.rust.size_memory)(input) };
        assert_eq!(rust_result, c_result, "CONFIGS.md row 1, input {input}");
    }
}

#[test]
fn configuration_rows_2_through_45_validate_match() {
    let pair = Pair::load();
    for config in configurations() {
        let mut rng = XorShift64(FIXED_SEED ^ config.row as u64);
        for iteration in 0..RANDOM_CASES_PER_CONFIG {
            let input = valid_input(config, iteration, &mut rng);
            pair.validate(
                &input,
                &format!(
                    "CONFIGS.md row {} ({:?} + {:?} + {:?}), iteration {}",
                    config.row, config.mode, config.rice, config.partition, iteration
                ),
            );
        }
    }
}

#[test]
fn error_rows_1_through_11_match_exactly() {
    let pair = Pair::load();
    let cases = [
        (1, with_u32(baseline(), 0, 0)),
        (1, with_u32(baseline(), 0, 15)),
        (2, with_u32(baseline(), 0, 65_536)),
        (2, with_u32(baseline(), 0, u32::MAX)),
        (3, with_u32(baseline(), 4, 0)),
        (4, with_u32(baseline(), 4, 655_351)),
        (4, with_u32(baseline(), 4, u32::MAX)),
        (5, with_u32(baseline(), 8, 0)),
        (6, with_u32(baseline(), 8, 9)),
        (6, with_u32(baseline(), 8, u32::MAX)),
        (7, with_u32(baseline(), 12, 0)),
        (8, with_u32(baseline(), 12, 33)),
        (8, with_u32(baseline(), 12, u32::MAX)),
        (9, with_u8(baseline(), 17, 31)),
        (9, with_u8(baseline(), 17, u8::MAX)),
        (10, with_u8(baseline(), 19, 16)),
        (10, with_u8(baseline(), 19, u8::MAX)),
        (11, with_u8(with_u8(baseline(), 18, 1), 19, 0)),
        (11, with_u8(with_u8(baseline(), 18, 15), 19, 14)),
    ];

    for (row, input) in cases {
        let mut c_value = input.clone();
        let mut rust_value = input.clone();
        let c_result = unsafe { (pair.c.validate)(&mut c_value) };
        let rust_result = unsafe { (pair.rust.validate)(&mut rust_value) };
        assert_eq!(c_result, -1, "ERRORS.md row {row}: C did not return -1");
        assert_eq!(
            rust_result, c_result,
            "ERRORS.md row {row}: return value differs"
        );
        assert_eq!(
            rust_value, c_value,
            "ERRORS.md row {row}: bytes differ after rejection"
        );
    }
}

#[test]
fn null_pointer_behavior_matches() {
    const PROBE_ENV: &str = "TFLAC_NULL_PROBE_LIBRARY";
    if let Ok(library) = std::env::var(PROBE_ENV) {
        let path = if library == "c" {
            c_library_path()
        } else {
            rust_library_path()
        };
        let api = unsafe { Api::load(&path) };
        let result = unsafe { (api.validate)(std::ptr::null_mut()) };
        std::process::exit((result as u8) as i32);
    }

    let executable = std::env::current_exe().expect("cannot locate test executable");
    let run_probe = |library: &str| {
        Command::new(&executable)
            .args(["--exact", "null_pointer_behavior_matches", "--nocapture"])
            .env(PROBE_ENV, library)
            .status()
            .unwrap_or_else(|error| panic!("failed to run {library} null probe: {error}"))
    };
    let c_status = run_probe("c");
    let rust_status = run_probe("rust");

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "null pointer termination signals differ: C={c_status:?}, Rust={rust_status:?}"
        );
        assert!(
            c_status.signal().is_some(),
            "C unexpectedly returned from flac_validate(NULL): {c_status:?}"
        );
    }
}
