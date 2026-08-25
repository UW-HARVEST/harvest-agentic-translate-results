use libloading::Library;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

type PackFn = unsafe extern "C" fn(*mut u8, u64);
type AddSampleFn = unsafe extern "C" fn(*mut Md5Bytes, u32, u64);
type UpdateFn = unsafe extern "C" fn(*mut TflacBytes, *const i32) -> u32;

const MD5_SIZE: usize = 88;
const TFLAC_SIZE: usize = 96;
const BUFFER_OFFSET: usize = 16;
const SAMPLE_COUNT: usize = 136;

#[repr(C, align(8))]
#[derive(Clone, Debug, PartialEq, Eq)]
struct Md5Bytes([u8; MD5_SIZE]);

#[repr(C, align(8))]
#[derive(Clone, Debug, PartialEq, Eq)]
struct TflacBytes([u8; TFLAC_SIZE]);

struct Api {
    _library: Library,
    pack: PackFn,
    add_sample: AddSampleFn,
    update: UpdateFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let pack = unsafe { *library.get::<PackFn>(b"tflac_pack_u64le\0").unwrap() };
        let add_sample = unsafe {
            *library
                .get::<AddSampleFn>(b"tflac_md5_addsample\0")
                .unwrap()
        };
        let update = unsafe { *library.get::<UpdateFn>(b"update_md5\0").unwrap() };
        Self {
            _library: library,
            pack,
            add_sample,
            update,
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
        let rust_path = root.join("target/release/libupdate_md5_lib.so");
        assert!(c_path.is_file(), "missing C library: {}", c_path.display());
        assert!(
            rust_path.is_file(),
            "missing Rust library: {}",
            rust_path.display()
        );
        unsafe {
            Self {
                c: Api::load(&c_path),
                rust: Api::load(&rust_path),
            }
        }
    }
}

fn library_path(kind: &OsStr) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    match kind.to_str().unwrap() {
        "c" => root.join("c_src/build/libtranslated_rust.so"),
        "rust" => root.join("target/release/libupdate_md5_lib.so"),
        other => panic!("unknown library kind {other}"),
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn u32(&mut self) -> u32 {
        self.u64() as u32
    }

    fn range(&mut self, start: u32, end_inclusive: u32) -> u32 {
        start + self.u32() % (end_inclusive - start + 1)
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.u64() as u8;
        }
    }
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_ne_bytes());
}

fn md5_input(rng: &mut Rng, pos: u32, total: u64) -> Md5Bytes {
    let mut result = Md5Bytes([0; MD5_SIZE]);
    rng.fill(&mut result.0);
    put_u32(&mut result.0, 0, pos);
    put_u64(&mut result.0, 8, total);
    result
}

fn tflac_input(
    rng: &mut Rng,
    pos: u32,
    total: u64,
    cur_blocksize: u32,
    channels: u32,
) -> TflacBytes {
    let mut result = TflacBytes([0; TFLAC_SIZE]);
    rng.fill(&mut result.0);
    put_u32(&mut result.0, 0, pos);
    put_u64(&mut result.0, 8, total);
    put_u32(&mut result.0, 88, cur_blocksize);
    put_u32(&mut result.0, 92, channels);
    result
}

fn samples(rng: &mut Rng) -> Vec<i32> {
    let mut values = Vec::with_capacity(SAMPLE_COUNT);
    for index in 0..SAMPLE_COUNT {
        let random = rng.u32();
        let value = match index % 5 {
            0 => random as i32,
            1 => (random | 0x8000_0000) as i32,
            2 => (random & 0xff) as i32,
            3 => ((random & 0xff) | 0x7fff_0000) as i32,
            _ => (!(random & 0xff)) as i32,
        };
        values.push(value);
    }
    values
}

fn compare_pack(libraries: &Libraries, value: u64, rng: &mut Rng) {
    let mut c_output = [0_u8; 16];
    rng.fill(&mut c_output);
    let mut rust_output = c_output;
    unsafe {
        (libraries.c.pack)(c_output.as_mut_ptr().add(4), value);
        (libraries.rust.pack)(rust_output.as_mut_ptr().add(4), value);
    }
    assert_eq!(
        c_output, rust_output,
        "pack mismatch for value {value:#018x}"
    );
}

fn compare_add(libraries: &Libraries, input: Md5Bytes, bits: u32, value: u64) {
    let mut c_output = input.clone();
    let mut rust_output = input;
    unsafe {
        (libraries.c.add_sample)(&mut c_output, bits, value);
        (libraries.rust.add_sample)(&mut rust_output, bits, value);
    }
    assert_eq!(
        c_output, rust_output,
        "addsample mismatch for bits={bits}, value={value:#018x}"
    );
}

fn compare_update(libraries: &Libraries, input: TflacBytes, sample_values: &[i32]) -> u32 {
    let mut c_output = input.clone();
    let mut rust_output = input;
    let c_result;
    let rust_result;
    unsafe {
        c_result = (libraries.c.update)(&mut c_output, sample_values.as_ptr());
        rust_result = (libraries.rust.update)(&mut rust_output, sample_values.as_ptr());
    }
    assert_eq!(c_result, rust_result, "update_md5 return mismatch");
    assert_eq!(c_output, rust_output, "update_md5 state mismatch");
    c_result
}

fn run_update_cases(
    seed: u64,
    positions: impl Fn(&mut Rng) -> u32,
    products: impl Fn(&mut Rng) -> (u32, u32),
) {
    let libraries = Libraries::load();
    let mut rng = Rng::new(seed);
    for iteration in 0..256 {
        let pos = positions(&mut rng);
        let (cur_blocksize, channels) = products(&mut rng);
        let total = if iteration % 7 == 0 {
            u64::MAX - 63
        } else {
            rng.u64()
        };
        let input = tflac_input(&mut rng, pos, total, cur_blocksize, channels);
        let sample_values = samples(&mut rng);
        compare_update(&libraries, input, &sample_values);
    }
}

fn product_at_least_40(rng: &mut Rng) -> (u32, u32) {
    let channels = rng.range(1, 16);
    let minimum = 40_u32.div_ceil(channels);
    (rng.range(minimum, minimum.saturating_add(10_000)), channels)
}

fn product_below_40(rng: &mut Rng) -> (u32, u32) {
    loop {
        let cur_blocksize = rng.range(0, 39);
        let channels = rng.range(0, 39);
        if cur_blocksize * channels < 40 {
            return (cur_blocksize, channels);
        }
    }
}

fn overflowing_product(rng: &mut Rng) -> (u32, u32) {
    let channels = rng.range(2, 65_535);
    let minimum = (u32::MAX as u64 / channels as u64 + 1) as u32;
    let cur_blocksize = minimum.saturating_add(rng.range(0, 10_000));
    assert!((cur_blocksize as u64) * (channels as u64) > u32::MAX as u64);
    (cur_blocksize, channels)
}

fn large_safe_position(rng: &mut Rng) -> u32 {
    rng.range(1, u32::MAX / 64) * 64 + rng.range(56, 63)
}

#[test]
fn config_01_pack_all_u64_shapes() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x01c0_ffee_1234_5678);
    for value in [0, 1, u64::MAX, 0x0123_4567_89ab_cdef] {
        compare_pack(&libraries, value, &mut rng);
    }
    for _ in 0..512 {
        let value = rng.u64();
        compare_pack(&libraries, value, &mut rng);
    }
}

#[test]
fn config_02_add_zero_bytes_no_rollover() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x02c0_ffee_1234_5678);
    for _ in 0..256 {
        let pos = rng.range(0, 56);
        let bits = rng.range(0, 7);
        let total = rng.u64();
        let input = md5_input(&mut rng, pos, total);
        compare_add(&libraries, input, bits, rng.u64());
    }
}

#[test]
fn config_03_add_positive_bytes_no_rollover() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x03c0_ffee_1234_5678);
    for iteration in 0..256 {
        let pos = rng.range(0, 55);
        let max_bytes = (63 - pos).min(7);
        let bytes = rng.range(1, max_bytes);
        let remainder = if iteration % 2 == 0 {
            0
        } else {
            rng.range(1, 7)
        };
        let bits = bytes * 8 + remainder;
        let total = rng.u64();
        let input = md5_input(&mut rng, pos, total);
        compare_add(&libraries, input, bits, rng.u64());
    }
}

#[test]
fn config_04_add_write_crosses_staging_tail_without_rollover() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x04c0_ffee_1234_5678);
    for _ in 0..256 {
        let pos = rng.range(57, 63);
        let bits = rng.range(0, 7);
        let total = rng.u64();
        let input = md5_input(&mut rng, pos, total);
        compare_add(&libraries, input, bits, rng.u64());
    }
}

#[test]
fn config_05_add_rollover_without_copy() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x05c0_ffee_1234_5678);
    for _ in 0..256 {
        let pos = rng.range(0, 63);
        let bytes = 64 - pos + 64 * rng.range(0, 100);
        let bits = bytes * 8 + rng.range(0, 7);
        let total = rng.u64();
        let input = md5_input(&mut rng, pos, total);
        compare_add(&libraries, input, bits, rng.u64());
    }
}

#[test]
fn config_06_add_rollover_with_one_copy() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x06c0_ffee_1234_5678);
    for _ in 0..256 {
        let pos = rng.range(0, 63);
        let bytes = 65 - pos + 64 * rng.range(0, 100);
        let bits = bytes * 8 + rng.range(0, 7);
        let total = rng.u64();
        let input = md5_input(&mut rng, pos, total);
        compare_add(&libraries, input, bits, rng.u64());
    }
}

#[test]
fn config_07_add_rollover_with_many_copies() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x07c0_ffee_1234_5678);
    for _ in 0..256 {
        let pos = rng.range(0, 63);
        let reduced = rng.range(2, 7);
        let bytes = 64 - pos + reduced + 64 * rng.range(0, 100);
        let bits = bytes * 8 + rng.range(0, 7);
        let total = rng.u64();
        let input = md5_input(&mut rng, pos, total);
        compare_add(&libraries, input, bits, rng.u64());
    }
}

#[test]
fn config_08_add_position_addition_wraps_below_64() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x08c0_ffee_1234_5678);
    for _ in 0..256 {
        let distance_to_wrap = rng.range(0, 10_000);
        let pos = u32::MAX - distance_to_wrap;
        let wrapped_pos = rng.range(0, 63);
        let bytes = distance_to_wrap + 1 + wrapped_pos;
        let bits = bytes * 8 + rng.range(0, 7);
        let total = rng.u64();
        let input = md5_input(&mut rng, pos, total);
        compare_add(&libraries, input, bits, rng.u64());
    }
}

#[test]
fn config_09_add_total_wraps() {
    let libraries = Libraries::load();
    let mut rng = Rng::new(0x09c0_ffee_1234_5678);
    for _ in 0..256 {
        let bits = rng.range(1, 64);
        let total = u64::MAX - u64::from(bits) + rng.range(1, bits.min(1024)) as u64;
        let pos = rng.range(0, 63);
        let input = md5_input(&mut rng, pos, total);
        compare_add(&libraries, input, bits, rng.u64());
    }
}

#[test]
fn config_10_update_no_rollover_product_at_least_40() {
    run_update_cases(
        0x10c0_ffee_1234_5678,
        |rng| rng.range(0, 23),
        product_at_least_40,
    );
}

#[test]
fn config_11_update_no_rollover_return_wraps() {
    run_update_cases(
        0x11c0_ffee_1234_5678,
        |rng| rng.range(0, 23),
        product_below_40,
    );
}

#[test]
fn config_12_update_no_rollover_product_wraps() {
    run_update_cases(
        0x12c0_ffee_1234_5678,
        |rng| rng.range(0, 23),
        overflowing_product,
    );
}

#[test]
fn config_13_update_one_rollover_product_at_least_40() {
    run_update_cases(
        0x13c0_ffee_1234_5678,
        |rng| rng.range(24, 63),
        product_at_least_40,
    );
}

#[test]
fn config_14_update_one_rollover_return_wraps() {
    run_update_cases(
        0x14c0_ffee_1234_5678,
        |rng| rng.range(24, 63),
        product_below_40,
    );
}

#[test]
fn config_15_update_one_rollover_product_wraps() {
    run_update_cases(
        0x15c0_ffee_1234_5678,
        |rng| rng.range(24, 63),
        overflowing_product,
    );
}

#[test]
fn config_16_update_normalizes_large_pos_product_at_least_40() {
    run_update_cases(
        0x16c0_ffee_1234_5678,
        large_safe_position,
        product_at_least_40,
    );
}

#[test]
fn config_17_update_normalizes_large_pos_return_wraps() {
    run_update_cases(0x17c0_ffee_1234_5678, large_safe_position, product_below_40);
}

#[test]
fn config_18_update_normalizes_large_pos_product_wraps() {
    run_update_cases(
        0x18c0_ffee_1234_5678,
        large_safe_position,
        overflowing_product,
    );
}

#[test]
fn abi_layout_matches_c_header() {
    assert_eq!(std::mem::size_of::<Md5Bytes>(), MD5_SIZE);
    assert_eq!(std::mem::align_of::<Md5Bytes>(), 8);
    assert_eq!(std::mem::size_of::<TflacBytes>(), TFLAC_SIZE);
    assert_eq!(std::mem::align_of::<TflacBytes>(), 8);
    assert_eq!(BUFFER_OFFSET, 16);
}

#[test]
fn null_pointer_child() {
    let Some(case) = std::env::var_os("DIFFERENTIAL_NULL_CASE") else {
        return;
    };
    let kind = std::env::var_os("DIFFERENTIAL_LIBRARY").unwrap();
    let api = unsafe { Api::load(&library_path(&kind)) };
    match case.to_str().unwrap() {
        "pack_destination" => unsafe { (api.pack)(std::ptr::null_mut(), 0) },
        "add_context" => unsafe { (api.add_sample)(std::ptr::null_mut(), 64, 0) },
        "update_context" => unsafe {
            (api.update)(std::ptr::null_mut(), std::ptr::null());
        },
        "update_samples" => {
            let mut state = TflacBytes([0; TFLAC_SIZE]);
            unsafe {
                (api.update)(&mut state, std::ptr::null());
            }
        }
        other => panic!("unknown null case {other}"),
    }
}

#[cfg(unix)]
#[test]
fn generic_null_pointer_boundaries_have_matching_process_result() {
    use std::os::unix::process::ExitStatusExt;

    let executable = std::env::current_exe().unwrap();
    for case in [
        "pack_destination",
        "add_context",
        "update_context",
        "update_samples",
    ] {
        let mut signals = Vec::new();
        for library in ["c", "rust"] {
            let status = Command::new(&executable)
                .args(["--exact", "null_pointer_child"])
                .env("DIFFERENTIAL_NULL_CASE", case)
                .env("DIFFERENTIAL_LIBRARY", library)
                .status()
                .unwrap();
            assert!(
                !status.success(),
                "{library} unexpectedly returned from null case {case}"
            );
            signals.push(status.signal());
        }
        assert_eq!(
            signals[0], signals[1],
            "C and Rust terminated differently for null case {case}"
        );
        assert!(signals[0].is_some(), "null case {case} did not signal");
    }
}
