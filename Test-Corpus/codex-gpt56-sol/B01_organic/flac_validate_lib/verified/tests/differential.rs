use libloading::Library;
use std::alloc::{Layout, alloc_zeroed, dealloc, handle_alloc_error};
use std::ffi::c_int;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::NonNull;
use std::slice;

const RANDOM_CASES_PER_CONFIG: usize = 64;
const RANDOM_SIZE_CASES: usize = 10_000;

#[repr(C)]
struct Tflac {
    blocksize: u32,
    samplerate: u32,
    channels: u32,
    bitdepth: u32,
    channel_mode: u8,
    max_rice_value: u8,
    min_partition_order: u8,
    max_partition_order: u8,
    partition_order: u8,
    cur_blocksize: u32,
}

struct TflacBuffer(NonNull<Tflac>);

impl TflacBuffer {
    fn zeroed() -> Self {
        let raw = unsafe { alloc_zeroed(Layout::new::<Tflac>()) }.cast::<Tflac>();
        Self(NonNull::new(raw).unwrap_or_else(|| handle_alloc_error(Layout::new::<Tflac>())))
    }

    fn duplicate(&self) -> Self {
        let duplicate = Self::zeroed();
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.0.as_ptr().cast::<u8>(),
                duplicate.0.as_ptr().cast::<u8>(),
                size_of::<Tflac>(),
            );
        }
        duplicate
    }

    fn as_mut_ptr(&mut self) -> *mut Tflac {
        self.0.as_ptr()
    }

    fn as_mut(&mut self) -> &mut Tflac {
        unsafe { self.0.as_mut() }
    }

    fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.0.as_ptr().cast::<u8>(), size_of::<Tflac>()) }
    }
}

impl Drop for TflacBuffer {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.0.as_ptr().cast::<u8>(), Layout::new::<Tflac>());
        }
    }
}

type ValidateFn = unsafe extern "C" fn(*mut Tflac) -> c_int;
type SizeMemoryFn = unsafe extern "C" fn(u32) -> u32;

struct DifferentialLibraries {
    _c: Library,
    _rust: Library,
    c_validate: ValidateFn,
    rust_validate: ValidateFn,
    c_size_memory: SizeMemoryFn,
    rust_size_memory: SizeMemoryFn,
}

impl DifferentialLibraries {
    fn load() -> Self {
        unsafe {
            let c = Library::new(c_library_path()).expect("load C shared library");
            let rust = Library::new(rust_library_path()).expect("load Rust shared library");
            let c_validate = *c
                .get::<ValidateFn>(b"flac_validate")
                .expect("load C flac_validate");
            let rust_validate = *rust
                .get::<ValidateFn>(b"flac_validate")
                .expect("load Rust flac_validate");
            let c_size_memory = *c
                .get::<SizeMemoryFn>(b"tflac_size_memory")
                .expect("load C tflac_size_memory");
            let rust_size_memory = *rust
                .get::<SizeMemoryFn>(b"tflac_size_memory")
                .expect("load Rust tflac_size_memory");
            Self {
                _c: c,
                _rust: rust,
                c_validate,
                rust_validate,
                c_size_memory,
                rust_size_memory,
            }
        }
    }

    fn compare_validate(&self, input: &TflacBuffer, context: &str) -> c_int {
        let mut c_value = input.duplicate();
        let mut rust_value = input.duplicate();
        let c_result = unsafe { (self.c_validate)(c_value.as_mut_ptr()) };
        let rust_result = unsafe { (self.rust_validate)(rust_value.as_mut_ptr()) };

        assert_eq!(
            rust_result, c_result,
            "{context}: return values differ (C={c_result}, Rust={rust_result})"
        );
        assert_eq!(
            rust_value.as_bytes(),
            c_value.as_bytes(),
            "{context}: output struct bytes differ"
        );
        c_result
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_library_path() -> PathBuf {
    let executable = std::env::current_exe().expect("current test executable");
    let deps_directory = executable.parent().expect("test deps directory");
    let profile_directory = deps_directory.parent().expect("Cargo profile directory");
    let build_path = profile_directory.join("libflac_validate_lib.so");
    if build_path.is_file() {
        build_path
    } else {
        deps_directory.join("libflac_validate_lib.so")
    }
}

struct FixedRng(u64);

impl FixedRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        (value.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }

    fn inclusive(&mut self, minimum: u32, maximum: u32) -> u32 {
        let width = u64::from(maximum) - u64::from(minimum) + 1;
        (u64::from(self.next_u32()) % width + u64::from(minimum)) as u32
    }
}

fn class_value(rng: &mut FixedRng, sample: usize, minimum: u32, maximum: u32) -> u32 {
    match sample {
        0 => minimum,
        1 => maximum,
        _ => rng.inclusive(minimum, maximum),
    }
}

fn non_stereo_channels(rng: &mut FixedRng, sample: usize) -> u32 {
    match sample {
        0 => 1,
        1 => 8,
        _ => loop {
            let channels = rng.inclusive(1, 8);
            if channels != 2 {
                break channels;
            }
        },
    }
}

fn partition_values(rng: &mut FixedRng, sample: usize, shape: usize) -> (u32, u8, u8) {
    match shape {
        // Equal bounds: the loop can never increment.
        0 => {
            let order = class_value(rng, sample, 0, 15) as u8;
            (class_value(rng, sample, 16, 65_535), order, order)
        }
        // The first divisibility check fails.
        1 => {
            let minimum = class_value(rng, sample, 0, 14) as u8;
            let maximum = rng.inclusive(u32::from(minimum) + 1, 15) as u8;
            let divisor = 1_u32 << (minimum + 1);
            let candidate = class_value(rng, sample, 16, 65_535);
            let blocksize = if candidate % divisor == 0 {
                candidate + 1
            } else {
                candidate
            };
            (blocksize, minimum, maximum)
        }
        // Divisibility remains true through the maximum order.
        2 => {
            let maximum = class_value(rng, sample, 1, 15) as u8;
            let minimum = rng.inclusive(0, u32::from(maximum) - 1) as u8;
            let divisor = 1_u32 << maximum;
            let minimum_multiple = 16_u32.div_ceil(divisor);
            let maximum_multiple = 65_535 / divisor;
            let multiple = rng.inclusive(minimum_multiple, maximum_multiple);
            (multiple * divisor, minimum, maximum)
        }
        // One increment succeeds and the next divisibility check fails.
        3 => {
            let minimum = class_value(rng, sample, 0, 13) as u8;
            let maximum = rng.inclusive(u32::from(minimum) + 2, 15) as u8;
            let divisor = 1_u32 << (minimum + 1);
            let first_multiplier = 16_u32.div_ceil(divisor) | 1;
            let last_multiplier = (65_535 / divisor) | 1;
            let multiplier = loop {
                let candidate = rng.inclusive(first_multiplier, last_multiplier);
                if candidate % 2 == 1 {
                    break candidate;
                }
            };
            (multiplier * divisor, minimum, maximum)
        }
        _ => unreachable!("unknown partition shape"),
    }
}

fn valid_input(
    rng: &mut FixedRng,
    sample: usize,
    depth_class: usize,
    channel_class: usize,
    mode_class: usize,
    rice_class: usize,
    partition_shape: usize,
) -> TflacBuffer {
    let bitdepth = match depth_class {
        0 => class_value(rng, sample, 1, 16),
        1 => class_value(rng, sample, 17, 31),
        2 => 32,
        _ => unreachable!("unknown bit-depth class"),
    };
    let channels = match channel_class {
        0 => 2,
        1 => non_stereo_channels(rng, sample),
        _ => unreachable!("unknown channel class"),
    };
    let channel_mode = match mode_class {
        0 => 0,
        1 => class_value(rng, sample, 1, 3) as u8,
        2 => class_value(rng, sample, 4, 255) as u8,
        _ => unreachable!("unknown channel-mode class"),
    };
    let max_rice_value = match rice_class {
        0 => 0,
        1 => class_value(rng, sample, 1, 30) as u8,
        _ => unreachable!("unknown Rice class"),
    };
    let (blocksize, min_partition_order, max_partition_order) =
        partition_values(rng, sample, partition_shape);

    let mut value = TflacBuffer::zeroed();
    let fields = value.as_mut();
    fields.blocksize = blocksize;
    fields.samplerate = class_value(rng, sample, 1, 655_350);
    fields.channels = channels;
    fields.bitdepth = bitdepth;
    fields.channel_mode = channel_mode;
    fields.max_rice_value = max_rice_value;
    fields.min_partition_order = min_partition_order;
    fields.max_partition_order = max_partition_order;
    fields.partition_order = rng.next_u32() as u8;
    fields.cur_blocksize = rng.next_u32();
    value
}

fn baseline_input(rng: &mut FixedRng, sample: usize) -> TflacBuffer {
    valid_input(rng, sample, 1, 0, 2, 0, 2)
}

#[test]
fn size_memory_matches_for_config_row_1() {
    let libraries = DifferentialLibraries::load();
    let fixed = [
        0,
        1,
        3,
        4,
        15,
        16,
        65_535,
        1_073_741_819,
        1_073_741_820,
        1_073_741_823,
        1_073_741_824,
        u32::MAX - 1,
        u32::MAX,
    ];
    for blocksize in fixed {
        let c_result = unsafe { (libraries.c_size_memory)(blocksize) };
        let rust_result = unsafe { (libraries.rust_size_memory)(blocksize) };
        assert_eq!(rust_result, c_result, "blocksize={blocksize}");
    }

    let mut rng = FixedRng::new(0x9eba_7c15_4a11_c0de);
    for sample in 0..RANDOM_SIZE_CASES {
        let blocksize = rng.next_u32();
        let c_result = unsafe { (libraries.c_size_memory)(blocksize) };
        let rust_result = unsafe { (libraries.rust_size_memory)(blocksize) };
        assert_eq!(
            rust_result, c_result,
            "sample={sample}, blocksize={blocksize}"
        );
    }
}

#[test]
fn valid_validator_cross_product_matches_config_rows_2_through_145() {
    assert_eq!(
        size_of::<Tflac>(),
        28,
        "unexpected C-compatible struct size"
    );
    let libraries = DifferentialLibraries::load();
    let mut rng = FixedRng::new(0x6d2b_79f5_aa55_0123);
    let mut rows_exercised = 0;

    for depth_class in 0..3 {
        for channel_class in 0..2 {
            for mode_class in 0..3 {
                for rice_class in 0..2 {
                    for partition_shape in 0..4 {
                        let row = 2
                            + (((depth_class * 2 + channel_class) * 3 + mode_class) * 2
                                + rice_class)
                                * 4
                            + partition_shape;
                        rows_exercised += 1;
                        for sample in 0..RANDOM_CASES_PER_CONFIG {
                            let input = valid_input(
                                &mut rng,
                                sample,
                                depth_class,
                                channel_class,
                                mode_class,
                                rice_class,
                                partition_shape,
                            );
                            let result = libraries.compare_validate(
                                &input,
                                &format!("CONFIGS.md row {row}, sample {sample}"),
                            );
                            assert_eq!(result, 0, "CONFIGS.md row {row}, sample {sample}");
                        }
                    }
                }
            }
        }
    }

    assert_eq!(rows_exercised, 144);
}

#[test]
fn explicit_error_surface_matches_rows_1_through_11() {
    let libraries = DifferentialLibraries::load();
    let mut rng = FixedRng::new(0xd1ff_e2e0_55aa_7711);

    for row in 1..=11 {
        for sample in 0..RANDOM_CASES_PER_CONFIG {
            let mut input = baseline_input(&mut rng, sample);
            let value = input.as_mut();
            match row {
                1 => value.blocksize = class_value(&mut rng, sample, 0, 15),
                2 => value.blocksize = class_value(&mut rng, sample, 65_536, u32::MAX),
                3 => value.samplerate = 0,
                4 => value.samplerate = class_value(&mut rng, sample, 655_351, u32::MAX),
                5 => value.channels = 0,
                6 => value.channels = class_value(&mut rng, sample, 9, u32::MAX),
                7 => value.bitdepth = 0,
                8 => value.bitdepth = class_value(&mut rng, sample, 33, u32::MAX),
                9 => value.max_rice_value = class_value(&mut rng, sample, 31, 255) as u8,
                10 => {
                    value.max_partition_order = class_value(&mut rng, sample, 16, 255) as u8;
                    value.min_partition_order = 0;
                }
                11 => {
                    value.max_partition_order = class_value(&mut rng, sample, 0, 15) as u8;
                    value.min_partition_order = match sample {
                        0 | 1 => value.max_partition_order + 1,
                        _ => rng.inclusive(u32::from(value.max_partition_order) + 1, 255) as u8,
                    };
                }
                _ => unreachable!(),
            }

            let result = libraries
                .compare_validate(&input, &format!("ERRORS.md row {row}, sample {sample}"));
            assert_eq!(result, -1, "ERRORS.md row {row}, sample {sample}");
        }
    }
}

#[test]
fn null_pointer_probe() {
    let Ok(path) = std::env::var("FLAC_VALIDATE_NULL_PROBE_LIBRARY") else {
        return;
    };
    let library = unsafe { Library::new(path).expect("load null-probe library") };
    let validate = unsafe {
        *library
            .get::<ValidateFn>(b"flac_validate")
            .expect("load null-probe flac_validate")
    };
    unsafe {
        validate(std::ptr::null_mut());
    }
    panic!("flac_validate unexpectedly returned for a null pointer");
}

#[test]
fn null_pointer_boundary_matches_in_isolated_processes() {
    let executable = std::env::current_exe().expect("current test executable");
    let probe = |library: PathBuf| {
        Command::new(&executable)
            .arg("--exact")
            .arg("null_pointer_probe")
            .arg("--nocapture")
            .env("FLAC_VALIDATE_NULL_PROBE_LIBRARY", library)
            .status()
            .expect("run null-pointer probe")
    };

    let c_status = probe(c_library_path());
    let rust_status = probe(rust_library_path());
    assert!(
        !c_status.success(),
        "C unexpectedly accepted a null pointer"
    );
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "C and Rust null-pointer termination signals differ"
    );
}
