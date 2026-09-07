use libloading::Library;
use std::ffi::c_int;
use std::mem::{size_of, zeroed};
use std::path::PathBuf;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

type Validate = unsafe extern "C" fn(*mut Tflac) -> c_int;
type SizeMemory = unsafe extern "C" fn(u32) -> u32;

struct Apis {
    _c_library: Library,
    _rust_library: Library,
    c_validate: Validate,
    rust_validate: Validate,
    c_size_memory: SizeMemory,
    rust_size_memory: SizeMemory,
}

impl Apis {
    fn load() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("../c_src/build/libharvest-work-QAS72x.so");
        let rust_path = root.join("target/release/libflac_validate_lib.so");
        assert!(
            c_path.is_file(),
            "missing C shared library: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "missing release Rust shared library: {}",
            rust_path.display()
        );

        unsafe {
            let c_library = Library::new(&c_path).expect("load C shared library");
            let rust_library = Library::new(&rust_path).expect("load Rust shared library");
            let c_validate = *c_library
                .get::<Validate>(b"flac_validate\0")
                .expect("load C flac_validate");
            let rust_validate = *rust_library
                .get::<Validate>(b"flac_validate\0")
                .expect("load Rust flac_validate");
            let c_size_memory = *c_library
                .get::<SizeMemory>(b"tflac_size_memory\0")
                .expect("load C tflac_size_memory");
            let rust_size_memory = *rust_library
                .get::<SizeMemory>(b"tflac_size_memory\0")
                .expect("load Rust tflac_size_memory");
            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_validate,
                rust_validate,
                c_size_memory,
                rust_size_memory,
            }
        }
    }
}

#[derive(Clone, Debug)]
struct CallResult {
    return_value: c_int,
    c_value: Tflac,
    c_bytes: Vec<u8>,
}

fn blank() -> Tflac {
    // All fields are integer types, so the all-zero representation is valid.
    unsafe { zeroed() }
}

fn duplicate(value: &Tflac) -> Tflac {
    let mut result = blank();
    result.blocksize = value.blocksize;
    result.samplerate = value.samplerate;
    result.channels = value.channels;
    result.bitdepth = value.bitdepth;
    result.channel_mode = value.channel_mode;
    result.max_rice_value = value.max_rice_value;
    result.min_partition_order = value.min_partition_order;
    result.max_partition_order = value.max_partition_order;
    result.partition_order = value.partition_order;
    result.cur_blocksize = value.cur_blocksize;
    result
}

fn object_bytes(value: &Tflac) -> Vec<u8> {
    unsafe {
        std::slice::from_raw_parts((value as *const Tflac).cast::<u8>(), size_of::<Tflac>())
            .to_vec()
    }
}

fn assert_validate_match(apis: &Apis, input: &Tflac, context: &str) -> CallResult {
    let mut c_value = duplicate(input);
    let mut rust_value = duplicate(input);
    let c_return = unsafe { (apis.c_validate)(&mut c_value) };
    let rust_return = unsafe { (apis.rust_validate)(&mut rust_value) };
    let c_bytes = object_bytes(&c_value);
    let rust_bytes = object_bytes(&rust_value);

    assert_eq!(
        rust_return, c_return,
        "return mismatch for {context}; input={input:?}"
    );
    assert_eq!(
        rust_bytes, c_bytes,
        "post-call byte mismatch for {context}; input={input:?}; C={c_value:?}; Rust={rust_value:?}"
    );
    CallResult {
        return_value: c_return,
        c_value,
        c_bytes,
    }
}

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 16) as u32
    }

    fn inclusive(&mut self, low: u32, high: u32) -> u32 {
        assert!(low <= high);
        low + self.next_u32() % (high - low + 1)
    }

    fn byte(&mut self) -> u8 {
        self.next_u32() as u8
    }
}

fn random_valid(rng: &mut Rng) -> Tflac {
    let mut value = blank();
    value.blocksize = rng.inclusive(16, 65_535);
    value.samplerate = rng.inclusive(1, 655_350);
    value.channels = rng.inclusive(1, 8);
    value.bitdepth = rng.inclusive(1, 32);
    value.channel_mode = rng.byte();
    value.max_rice_value = rng.inclusive(0, 30) as u8;
    value.min_partition_order = rng.inclusive(0, 15) as u8;
    value.max_partition_order = rng.inclusive(value.min_partition_order as u32, 15) as u8;
    value.partition_order = rng.byte();
    value.cur_blocksize = rng.next_u32();
    value
}

fn partition_fixed(value: &mut Tflac, rng: &mut Rng) {
    let order = rng.inclusive(0, 15) as u8;
    value.min_partition_order = order;
    value.max_partition_order = order;
}

fn partition_reaches_max(value: &mut Tflac, rng: &mut Rng) {
    let max = rng.inclusive(1, 15);
    let min = rng.inclusive(0, max - 1);
    let unit = 1_u32 << max;
    let low_multiple = 16_u32.div_ceil(unit);
    let high_multiple = 65_535 / unit;
    let multiple = rng.inclusive(low_multiple.max(1), high_multiple);
    value.blocksize = unit * multiple;
    value.min_partition_order = min as u8;
    value.max_partition_order = max as u8;
}

fn partition_stops_partway(value: &mut Tflac, rng: &mut Rng) {
    let max = rng.inclusive(2, 15);
    let stop = rng.inclusive(1, max - 1);
    let min = rng.inclusive(0, stop - 1);
    let unit = 1_u32 << stop;
    let low_multiple = 16_u32.div_ceil(unit);
    let high_multiple = 65_535 / unit;
    let mut odd_multiple = rng.inclusive(low_multiple.max(1), high_multiple) | 1;
    if odd_multiple > high_multiple {
        odd_multiple -= 2;
    }
    value.blocksize = unit * odd_multiple;
    value.min_partition_order = min as u8;
    value.max_partition_order = max as u8;
}

fn partition_fails_initially(value: &mut Tflac, rng: &mut Rng) {
    let min = rng.inclusive(0, 14);
    let max = rng.inclusive(min + 1, 15);
    let divisor = 1_u32 << (min + 1);
    loop {
        let blocksize = rng.inclusive(16, 65_535);
        if blocksize % divisor != 0 {
            value.blocksize = blocksize;
            break;
        }
    }
    value.min_partition_order = min as u8;
    value.max_partition_order = max as u8;
}

fn run_randomized(
    seed: u64,
    mut configure: impl FnMut(&mut Tflac, &mut Rng),
    mut verify: impl FnMut(&Tflac, &CallResult),
) {
    let apis = Apis::load();
    let mut rng = Rng::new(seed);
    for iteration in 0..256 {
        let mut input = random_valid(&mut rng);
        configure(&mut input, &mut rng);
        let result = assert_validate_match(&apis, &input, &format!("iteration {iteration}"));
        verify(&input, &result);
    }
}

fn assert_valid(input: &Tflac, result: &CallResult) {
    assert_eq!(result.return_value, 0, "C rejected valid input: {input:?}");
    assert_eq!(result.c_value.cur_blocksize, input.blocksize);
}

#[test]
fn abi_layout_matches_c_header() {
    assert_eq!(size_of::<Tflac>(), 28);
    assert_eq!(std::mem::align_of::<Tflac>(), 4);
}

#[test]
fn config_01_size_memory_all_u32_shapes() {
    let apis = Apis::load();
    let mut values = vec![
        0,
        1,
        15,
        16,
        65_535,
        (1_u32 << 30) - 4,
        (1_u32 << 30) - 1,
        1_u32 << 30,
        (1_u32 << 30) + 1,
        u32::MAX - 1,
        u32::MAX,
    ];
    let mut rng = Rng::new(0x0101_5eed);
    values.extend((0..4096).map(|_| rng.next_u32()));
    for blocksize in values {
        let c = unsafe { (apis.c_size_memory)(blocksize) };
        let rust = unsafe { (apis.rust_size_memory)(blocksize) };
        assert_eq!(rust, c, "blocksize={blocksize}");
    }
}

#[test]
fn config_02_independent_explicit_rice_fixed_partition() {
    run_randomized(
        0x0202_5eed,
        |value, rng| {
            value.channel_mode = 0;
            value.max_rice_value = rng.inclusive(1, 30) as u8;
            partition_fixed(value, rng);
        },
        assert_valid,
    );
}

#[test]
fn config_03_independent_explicit_rice_reaches_max() {
    run_randomized(
        0x0303_5eed,
        |value, rng| {
            value.channel_mode = 0;
            value.max_rice_value = rng.inclusive(1, 30) as u8;
            partition_reaches_max(value, rng);
        },
        |input, result| {
            assert_valid(input, result);
            assert_eq!(result.c_value.partition_order, input.max_partition_order);
        },
    );
}

#[test]
fn config_04_independent_explicit_rice_stops_partway() {
    run_randomized(
        0x0404_5eed,
        |value, rng| {
            value.channel_mode = 0;
            value.max_rice_value = rng.inclusive(1, 30) as u8;
            partition_stops_partway(value, rng);
        },
        |input, result| {
            assert_valid(input, result);
            assert!(result.c_value.partition_order < input.max_partition_order);
            assert!(result.c_value.partition_order > input.min_partition_order);
        },
    );
}

#[test]
fn config_05_independent_default_rice_14_fixed() {
    run_randomized(
        0x0505_5eed,
        |value, rng| {
            value.channel_mode = 0;
            value.bitdepth = rng.inclusive(1, 16);
            value.max_rice_value = 0;
            partition_fixed(value, rng);
        },
        |input, result| {
            assert_valid(input, result);
            assert_eq!(result.c_value.max_rice_value, 14);
        },
    );
}

#[test]
fn config_06_independent_default_rice_14_reaches_max() {
    run_randomized(
        0x0606_5eed,
        |value, rng| {
            value.channel_mode = 0;
            value.bitdepth = rng.inclusive(1, 16);
            value.max_rice_value = 0;
            partition_reaches_max(value, rng);
        },
        |input, result| {
            assert_valid(input, result);
            assert_eq!(result.c_value.max_rice_value, 14);
            assert_eq!(result.c_value.partition_order, input.max_partition_order);
        },
    );
}

#[test]
fn config_07_independent_default_rice_14_stops_partway() {
    run_randomized(
        0x0707_5eed,
        |value, rng| {
            value.channel_mode = 0;
            value.bitdepth = rng.inclusive(1, 16);
            value.max_rice_value = 0;
            partition_stops_partway(value, rng);
        },
        |input, result| {
            assert_valid(input, result);
            assert_eq!(result.c_value.max_rice_value, 14);
            assert!(result.c_value.partition_order < input.max_partition_order);
        },
    );
}

#[test]
fn config_08_independent_default_rice_30_fixed() {
    run_randomized(
        0x0808_5eed,
        |value, rng| {
            value.channel_mode = 0;
            value.bitdepth = rng.inclusive(17, 32);
            value.max_rice_value = 0;
            partition_fixed(value, rng);
        },
        |input, result| {
            assert_valid(input, result);
            assert_eq!(result.c_value.max_rice_value, 30);
        },
    );
}

#[test]
fn config_09_independent_default_rice_30_reaches_max() {
    run_randomized(
        0x0909_5eed,
        |value, rng| {
            value.channel_mode = 0;
            value.bitdepth = rng.inclusive(17, 32);
            value.max_rice_value = 0;
            partition_reaches_max(value, rng);
        },
        |input, result| {
            assert_valid(input, result);
            assert_eq!(result.c_value.max_rice_value, 30);
            assert_eq!(result.c_value.partition_order, input.max_partition_order);
        },
    );
}

#[test]
fn config_10_independent_default_rice_30_stops_partway() {
    run_randomized(
        0x1010_5eed,
        |value, rng| {
            value.channel_mode = 0;
            value.bitdepth = rng.inclusive(17, 32);
            value.max_rice_value = 0;
            partition_stops_partway(value, rng);
        },
        |input, result| {
            assert_valid(input, result);
            assert_eq!(result.c_value.max_rice_value, 30);
            assert!(result.c_value.partition_order < input.max_partition_order);
        },
    );
}

#[test]
fn config_11_valid_nonzero_mode_preserved_fixed() {
    run_randomized(
        0x1111_5eed,
        |value, rng| {
            value.channel_mode = rng.inclusive(1, 3) as u8;
            value.channels = 2;
            value.bitdepth = rng.inclusive(1, 31);
            value.max_rice_value = rng.inclusive(1, 30) as u8;
            partition_fixed(value, rng);
        },
        |input, result| {
            assert_valid(input, result);
            assert_eq!(result.c_value.channel_mode, input.channel_mode);
        },
    );
}

#[test]
fn config_12_valid_nonzero_mode_preserved_reaches_max() {
    run_randomized(
        0x1212_5eed,
        |value, rng| {
            value.channel_mode = rng.inclusive(1, 3) as u8;
            value.channels = 2;
            value.bitdepth = rng.inclusive(1, 31);
            value.max_rice_value = rng.inclusive(1, 30) as u8;
            partition_reaches_max(value, rng);
        },
        |input, result| {
            assert_valid(input, result);
            assert_eq!(result.c_value.channel_mode, input.channel_mode);
            assert_eq!(result.c_value.partition_order, input.max_partition_order);
        },
    );
}

#[test]
fn config_13_valid_nonzero_mode_preserved_stops_partway() {
    run_randomized(
        0x1313_5eed,
        |value, rng| {
            value.channel_mode = rng.inclusive(1, 3) as u8;
            value.channels = 2;
            value.bitdepth = rng.inclusive(1, 31);
            value.max_rice_value = rng.inclusive(1, 30) as u8;
            partition_stops_partway(value, rng);
        },
        |input, result| {
            assert_valid(input, result);
            assert_eq!(result.c_value.channel_mode, input.channel_mode);
            assert!(result.c_value.partition_order < input.max_partition_order);
        },
    );
}

#[test]
fn config_14_out_of_range_mode_is_preserved() {
    run_randomized(
        0x1414_5eed,
        |value, rng| {
            value.channel_mode = rng.inclusive(4, 255) as u8;
            value.channels = 2;
            value.bitdepth = rng.inclusive(1, 31);
            value.max_rice_value = if rng.next_u32() & 1 == 0 {
                0
            } else {
                rng.inclusive(1, 30) as u8
            };
            match rng.next_u32() % 4 {
                0 => partition_fixed(value, rng),
                1 => partition_reaches_max(value, rng),
                2 => partition_stops_partway(value, rng),
                _ => partition_fails_initially(value, rng),
            }
        },
        |input, result| {
            assert_valid(input, result);
            assert_eq!(result.c_value.channel_mode, input.channel_mode);
        },
    );
}

#[test]
fn config_15_nonzero_mode_resets_for_non_stereo() {
    run_randomized(
        0x1515_5eed,
        |value, rng| {
            value.channel_mode = rng.inclusive(1, 255) as u8;
            value.channels = loop {
                let channels = rng.inclusive(1, 8);
                if channels != 2 {
                    break channels;
                }
            };
            value.max_rice_value = if rng.next_u32() & 1 == 0 {
                0
            } else {
                rng.inclusive(1, 30) as u8
            };
            match rng.next_u32() % 4 {
                0 => partition_fixed(value, rng),
                1 => partition_reaches_max(value, rng),
                2 => partition_stops_partway(value, rng),
                _ => partition_fails_initially(value, rng),
            }
        },
        |input, result| {
            assert_valid(input, result);
            assert_eq!(result.c_value.channel_mode, 0);
        },
    );
}

#[test]
fn config_16_nonzero_mode_resets_for_32_bit_stereo() {
    run_randomized(
        0x1616_5eed,
        |value, rng| {
            value.channel_mode = rng.inclusive(1, 255) as u8;
            value.channels = 2;
            value.bitdepth = 32;
            value.max_rice_value = if rng.next_u32() & 1 == 0 {
                0
            } else {
                rng.inclusive(1, 30) as u8
            };
            match rng.next_u32() % 4 {
                0 => partition_fixed(value, rng),
                1 => partition_reaches_max(value, rng),
                2 => partition_stops_partway(value, rng),
                _ => partition_fails_initially(value, rng),
            }
        },
        |input, result| {
            assert_valid(input, result);
            assert_eq!(result.c_value.channel_mode, 0);
        },
    );
}

#[test]
fn config_17_valid_scalar_lower_boundaries() {
    run_randomized(
        0x1717_5eed,
        |value, rng| {
            value.blocksize = 16;
            value.samplerate = 1;
            value.channels = 1;
            value.bitdepth = 1;
            value.channel_mode = rng.byte();
            value.max_rice_value = rng.inclusive(0, 30) as u8;
            value.partition_order = rng.byte();
            value.cur_blocksize = rng.next_u32();
            match rng.next_u32() % 2 {
                0 => partition_fixed(value, rng),
                _ => partition_fails_initially(value, rng),
            }
        },
        assert_valid,
    );
}

#[test]
fn config_18_valid_scalar_upper_boundaries() {
    run_randomized(
        0x1818_5eed,
        |value, rng| {
            value.blocksize = 65_535;
            value.samplerate = 655_350;
            value.channels = 8;
            value.bitdepth = 32;
            value.channel_mode = rng.byte();
            value.max_rice_value = rng.inclusive(0, 30) as u8;
            value.min_partition_order = rng.inclusive(0, 15) as u8;
            value.max_partition_order = rng.inclusive(value.min_partition_order as u32, 15) as u8;
            value.partition_order = rng.byte();
            value.cur_blocksize = rng.next_u32();
        },
        |input, result| {
            assert_valid(input, result);
            assert_eq!(
                result.c_value.partition_order, input.min_partition_order,
                "65535 is odd, so the first divisor always fails"
            );
        },
    );
}

#[test]
fn config_19_partition_order_boundaries() {
    let apis = Apis::load();
    let mut cases = Vec::new();

    let mut odd = blank();
    odd.blocksize = 65_535;
    odd.samplerate = 1;
    odd.channels = 2;
    odd.bitdepth = 16;
    odd.max_rice_value = 1;
    odd.min_partition_order = 0;
    odd.max_partition_order = 15;
    cases.push((odd, 0));

    let mut power_of_two = duplicate(&odd);
    power_of_two.blocksize = 32_768;
    cases.push((power_of_two, 15));

    let mut sixteen = duplicate(&odd);
    sixteen.blocksize = 16;
    cases.push((sixteen, 4));

    let mut fixed_at_fifteen = duplicate(&odd);
    fixed_at_fifteen.min_partition_order = 15;
    fixed_at_fifteen.max_partition_order = 15;
    cases.push((fixed_at_fifteen, 15));

    for (input, expected_order) in cases {
        let result = assert_validate_match(&apis, &input, "partition boundary case");
        assert_valid(&input, &result);
        assert_eq!(result.c_value.partition_order, expected_order);
    }
}

#[test]
fn config_20_partition_fails_first_divisor() {
    run_randomized(0x2020_5eed, partition_fails_initially, |input, result| {
        assert_valid(input, result);
        assert_eq!(result.c_value.partition_order, input.min_partition_order);
    });
}

fn run_error_randomized(
    seed: u64,
    mut configure: impl FnMut(&mut Tflac, &mut Rng),
    mut verify: impl FnMut(&Tflac, &CallResult),
) {
    let apis = Apis::load();
    let mut rng = Rng::new(seed);
    for iteration in 0..128 {
        let mut input = random_valid(&mut rng);
        configure(&mut input, &mut rng);
        let result = assert_validate_match(&apis, &input, &format!("error iteration {iteration}"));
        assert_eq!(
            result.return_value, -1,
            "C accepted invalid input: {input:?}"
        );
        verify(&input, &result);
    }
}

fn assert_unchanged(input: &Tflac, result: &CallResult) {
    assert_eq!(result.c_bytes, object_bytes(input));
}

#[test]
fn error_01_blocksize_below_16() {
    run_error_randomized(
        0xe001_5eed,
        |value, rng| value.blocksize = rng.inclusive(0, 15),
        assert_unchanged,
    );
}

#[test]
fn error_02_blocksize_above_65535() {
    run_error_randomized(
        0xe002_5eed,
        |value, rng| value.blocksize = rng.inclusive(65_536, u32::MAX),
        assert_unchanged,
    );
}

#[test]
fn error_03_samplerate_zero() {
    run_error_randomized(
        0xe003_5eed,
        |value, _| value.samplerate = 0,
        assert_unchanged,
    );
}

#[test]
fn error_04_samplerate_above_655350() {
    run_error_randomized(
        0xe004_5eed,
        |value, rng| value.samplerate = rng.inclusive(655_351, u32::MAX),
        assert_unchanged,
    );
}

#[test]
fn error_05_channels_zero() {
    run_error_randomized(0xe005_5eed, |value, _| value.channels = 0, assert_unchanged);
}

#[test]
fn error_06_channels_above_8() {
    run_error_randomized(
        0xe006_5eed,
        |value, rng| value.channels = rng.inclusive(9, u32::MAX),
        assert_unchanged,
    );
}

#[test]
fn error_07_bitdepth_zero() {
    run_error_randomized(0xe007_5eed, |value, _| value.bitdepth = 0, assert_unchanged);
}

#[test]
fn error_08_bitdepth_above_32() {
    run_error_randomized(
        0xe008_5eed,
        |value, rng| value.bitdepth = rng.inclusive(33, u32::MAX),
        assert_unchanged,
    );
}

#[test]
fn error_09_max_rice_above_30_with_prior_mutation() {
    run_error_randomized(
        0xe009_5eed,
        |value, rng| {
            value.max_rice_value = rng.inclusive(31, 255) as u8;
            if rng.next_u32() & 1 == 0 {
                value.channel_mode = rng.inclusive(1, 255) as u8;
                value.channels = 1;
            } else {
                value.channel_mode = 0;
            }
        },
        |input, result| {
            let mut expected = duplicate(input);
            if expected.channel_mode != 0 && (expected.channels != 2 || expected.bitdepth == 32) {
                expected.channel_mode = 0;
            }
            assert_eq!(result.c_bytes, object_bytes(&expected));
        },
    );
}

#[test]
fn error_10_max_partition_above_15_with_prior_mutations() {
    run_error_randomized(
        0xe010_5eed,
        |value, rng| {
            value.channel_mode = rng.inclusive(1, 255) as u8;
            value.channels = 1;
            value.max_rice_value = if rng.next_u32() & 1 == 0 {
                0
            } else {
                rng.inclusive(1, 30) as u8
            };
            value.max_partition_order = rng.inclusive(16, 255) as u8;
        },
        |input, result| {
            let mut expected = duplicate(input);
            expected.channel_mode = 0;
            if expected.max_rice_value == 0 {
                expected.max_rice_value = if expected.bitdepth <= 16 { 14 } else { 30 };
            }
            assert_eq!(result.c_bytes, object_bytes(&expected));
        },
    );
}

#[test]
fn error_11_min_partition_above_max_with_prior_mutations() {
    run_error_randomized(
        0xe011_5eed,
        |value, rng| {
            value.channel_mode = rng.inclusive(1, 255) as u8;
            value.channels = 1;
            value.max_rice_value = 0;
            value.max_partition_order = rng.inclusive(0, 14) as u8;
            value.min_partition_order =
                rng.inclusive(value.max_partition_order as u32 + 1, 255) as u8;
        },
        |input, result| {
            let mut expected = duplicate(input);
            expected.channel_mode = 0;
            expected.max_rice_value = if expected.bitdepth <= 16 { 14 } else { 30 };
            assert_eq!(result.c_bytes, object_bytes(&expected));
        },
    );
}

#[test]
fn error_12_null_pointer_termination_matches() {
    let executable = std::env::current_exe().expect("current integration-test executable");
    let mut statuses = Vec::new();
    for library_kind in ["c", "rust"] {
        let status = Command::new(&executable)
            .args(["--exact", "null_pointer_child", "--ignored"])
            .env("TFLAC_NULL_CHILD_LIBRARY", library_kind)
            .status()
            .expect("run null-pointer child");
        assert!(
            !status.success(),
            "{library_kind} flac_validate unexpectedly accepted NULL"
        );
        statuses.push(status);
    }

    #[cfg(unix)]
    {
        assert_eq!(
            statuses[0].signal(),
            statuses[1].signal(),
            "C and Rust terminated with different signals: {statuses:?}"
        );
        assert!(
            statuses[0].signal().is_some(),
            "NULL call should terminate by signal: {statuses:?}"
        );
    }

    #[cfg(not(unix))]
    assert_eq!(statuses[0].code(), statuses[1].code());
}

#[test]
#[ignore = "spawned by error_12_null_pointer_termination_matches"]
fn null_pointer_child() {
    let Ok(library_kind) = std::env::var("TFLAC_NULL_CHILD_LIBRARY") else {
        return;
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = match library_kind.as_str() {
        "c" => root.join("../c_src/build/libharvest-work-QAS72x.so"),
        "rust" => root.join("target/release/libflac_validate_lib.so"),
        other => panic!("unknown child library kind: {other}"),
    };
    unsafe {
        let library = Library::new(path).expect("load child shared library");
        let validate = *library
            .get::<Validate>(b"flac_validate\0")
            .expect("load child flac_validate");
        let returned = validate(std::ptr::null_mut());
        panic!("NULL call unexpectedly returned {returned}");
    }
}
