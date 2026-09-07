use libloading::Library;
use std::ffi::{CStr, c_char, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;

type DropFn = unsafe extern "C" fn(*const c_char) -> *const c_char;
type FilterFn = unsafe extern "C" fn(*const c_char, bool) -> *mut c_char;

unsafe extern "C" {
    fn free(pointer: *mut c_void);
}

struct Api {
    _library: Library,
    drop_fn: DropFn,
    filter_fn: FilterFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let drop_fn = unsafe { *library.get::<DropFn>(b"w_utf8_drop\0").unwrap() };
        let filter_fn = unsafe { *library.get::<FilterFn>(b"w_utf8_filter\0").unwrap() };
        Self {
            _library: library,
            drop_fn,
            filter_fn,
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
                c: Api::load(&c_library()),
                rust: Api::load(&rust_library()),
            }
        }
    }

    fn compare(&self, bytes: &[u8]) -> Comparison {
        assert!(!bytes.contains(&0), "test input contains an interior NUL");
        let mut input = bytes.to_vec();
        input.push(0);
        let pointer = input.as_ptr().cast::<c_char>();

        let c_offset = unsafe { (self.c.drop_fn)(pointer).offset_from(pointer) };
        let rust_offset = unsafe { (self.rust.drop_fn)(pointer).offset_from(pointer) };
        assert_eq!(
            rust_offset,
            c_offset,
            "drop offset differs for input {}",
            hex(bytes)
        );

        let mut outputs = [Vec::new(), Vec::new()];
        for (index, replacement) in [false, true].into_iter().enumerate() {
            let c_output = unsafe { (self.c.filter_fn)(pointer, replacement) };
            let rust_output = unsafe { (self.rust.filter_fn)(pointer, replacement) };
            assert!(
                !c_output.is_null(),
                "C unexpectedly returned NULL for {}",
                hex(bytes)
            );
            assert!(
                !rust_output.is_null(),
                "Rust unexpectedly returned NULL for {}",
                hex(bytes)
            );

            let c_bytes = unsafe { CStr::from_ptr(c_output) }.to_bytes().to_vec();
            let rust_bytes = unsafe { CStr::from_ptr(rust_output) }.to_bytes().to_vec();
            unsafe {
                free(c_output.cast());
                free(rust_output.cast());
            }
            assert_eq!(
                rust_bytes,
                c_bytes,
                "filter output differs (replacement={replacement}) for {}",
                hex(bytes)
            );
            outputs[index] = c_bytes;
        }

        Comparison {
            drop_offset: c_offset as usize,
            delete_output: outputs[0].clone(),
            replace_output: outputs[1].clone(),
        }
    }
}

struct Comparison {
    drop_offset: usize,
    delete_output: Vec<u8>,
    replace_output: Vec<u8>,
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn range(&mut self, start: u8, end_inclusive: u8) -> u8 {
        start + (self.next() % u64::from(end_inclusive - start + 1)) as u8
    }

    fn usize(&mut self, start: usize, end_inclusive: usize) -> usize {
        start + (self.next() % (end_inclusive - start + 1) as u64) as usize
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library() -> PathBuf {
    manifest_dir().join("../c_src/build/libdriver.so")
}

fn rust_library() -> PathBuf {
    manifest_dir().join("target/release/libdriver.so")
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn assert_valid(pair: &Pair, bytes: &[u8]) {
    let result = pair.compare(bytes);
    assert_eq!(result.drop_offset, bytes.len(), "C rejected {}", hex(bytes));
    assert_eq!(result.delete_output, bytes);
    assert_eq!(result.replace_output, bytes);
}

fn valid_two(rng: &mut Rng) -> Vec<u8> {
    vec![rng.range(0xc2, 0xdf), rng.range(0x80, 0xbf)]
}

fn valid_three_ordinary(rng: &mut Rng) -> Vec<u8> {
    let choices = [
        0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xeb, 0xec, 0xee,
    ];
    vec![
        choices[rng.usize(0, choices.len() - 1)],
        rng.range(0x80, 0xbf),
        rng.range(0x80, 0xbf),
    ]
}

fn valid_four_ordinary(rng: &mut Rng) -> Vec<u8> {
    vec![
        rng.range(0xf1, 0xf3),
        rng.range(0x80, 0xbf),
        rng.range(0x80, 0xbf),
        rng.range(0x80, 0xbf),
    ]
}

fn append_random_valid(output: &mut Vec<u8>, rng: &mut Rng) {
    match rng.usize(0, 9) {
        0..=4 => output.push(rng.range(1, 0x7f)),
        5 => output.extend(valid_two(rng)),
        6 => output.extend(valid_three_ordinary(rng)),
        7 => output.extend([0xe0, rng.range(0xa0, 0xbf), rng.range(0x80, 0xbf)]),
        8 => output.extend(valid_four_ordinary(rng)),
        _ => output.extend([
            0xf4,
            rng.range(0x80, 0x8f),
            rng.range(0x80, 0xbf),
            rng.range(0x80, 0xbf),
        ]),
    }
}

fn invalid_shapes() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("continuation", vec![0x80]),
        ("overlong two-byte C0", vec![0xc0, 0x80]),
        ("overlong two-byte C1", vec![0xc1, 0xbf]),
        ("two-byte non-continuation", vec![0xc2, 0x41]),
        ("two-byte truncation", vec![0xdf]),
        ("three-byte second non-continuation", vec![0xe1, 0x41, 0x80]),
        ("three-byte second truncation", vec![0xe1]),
        ("three-byte third non-continuation", vec![0xe1, 0x80, 0x41]),
        ("three-byte third truncation", vec![0xe1, 0x80]),
        ("E0 overlong", vec![0xe0, 0x9f, 0x80]),
        ("ED surrogate", vec![0xed, 0xa0, 0x80]),
        (
            "four-byte second non-continuation",
            vec![0xf1, 0x41, 0x80, 0x80],
        ),
        ("four-byte second truncation", vec![0xf1]),
        (
            "four-byte third non-continuation",
            vec![0xf1, 0x80, 0x41, 0x80],
        ),
        ("four-byte third truncation", vec![0xf1, 0x80]),
        (
            "four-byte fourth non-continuation",
            vec![0xf1, 0x80, 0x80, 0x41],
        ),
        ("four-byte fourth truncation", vec![0xf1, 0x80, 0x80]),
        ("F0 overlong", vec![0xf0, 0x8f, 0x80, 0x80]),
        ("above U+10FFFF", vec![0xf4, 0x90, 0x80, 0x80]),
        ("lead F5", vec![0xf5, 0x80, 0x80, 0x80]),
        ("lead F7", vec![0xf7, 0xbf, 0xbf, 0xbf]),
        ("lead F8", vec![0xf8]),
        ("lead FF", vec![0xff]),
    ]
}

#[test]
fn config_01_empty_string() {
    let pair = Pair::load();
    assert_valid(&pair, &[]);
}

#[test]
fn config_02_ascii_one_and_many() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0202_0202);
    for _ in 0..256 {
        let bytes = (0..rng.usize(1, 512))
            .map(|_| rng.range(1, 0x7f))
            .collect::<Vec<_>>();
        assert_valid(&pair, &bytes);
    }
}

#[test]
fn config_03_valid_two_byte_sequences() {
    let pair = Pair::load();
    assert_valid(&pair, &[0xc2, 0x80]);
    assert_valid(&pair, &[0xdf, 0xbf]);
    let mut rng = Rng::new(0x0303_0303);
    for _ in 0..512 {
        assert_valid(&pair, &valid_two(&mut rng));
    }
}

#[test]
fn config_04_valid_ordinary_three_byte_sequences() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0404_0404);
    for _ in 0..512 {
        assert_valid(&pair, &valid_three_ordinary(&mut rng));
    }
}

#[test]
fn config_05_valid_e0_boundary() {
    let pair = Pair::load();
    assert_valid(&pair, &[0xe0, 0xa0, 0x80]);
    assert_valid(&pair, &[0xe0, 0xbf, 0xbf]);
    let mut rng = Rng::new(0x0505_0505);
    for _ in 0..512 {
        assert_valid(&pair, &[0xe0, rng.range(0xa0, 0xbf), rng.range(0x80, 0xbf)]);
    }
}

#[test]
fn config_06_valid_ed_boundary() {
    let pair = Pair::load();
    assert_valid(&pair, &[0xed, 0x80, 0x80]);
    assert_valid(&pair, &[0xed, 0x9f, 0xbf]);
    let mut rng = Rng::new(0x0606_0606);
    for _ in 0..512 {
        assert_valid(&pair, &[0xed, rng.range(0x80, 0x9f), rng.range(0x80, 0xbf)]);
    }
}

#[test]
fn config_07_valid_ef_boundary() {
    let pair = Pair::load();
    assert_valid(&pair, &[0xef, 0x80, 0x80]);
    assert_valid(&pair, &[0xef, 0xbf, 0xbf]);
    let mut rng = Rng::new(0x0707_0707);
    for _ in 0..512 {
        assert_valid(&pair, &[0xef, rng.range(0x80, 0xbf), rng.range(0x80, 0xbf)]);
    }
}

#[test]
fn config_08_valid_ordinary_four_byte_sequences() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0808_0808);
    for _ in 0..512 {
        assert_valid(&pair, &valid_four_ordinary(&mut rng));
    }
}

#[test]
fn config_09_valid_f0_boundary() {
    let pair = Pair::load();
    assert_valid(&pair, &[0xf0, 0x90, 0x80, 0x80]);
    assert_valid(&pair, &[0xf0, 0xbf, 0xbf, 0xbf]);
    let mut rng = Rng::new(0x0909_0909);
    for _ in 0..512 {
        assert_valid(
            &pair,
            &[
                0xf0,
                rng.range(0x90, 0xbf),
                rng.range(0x80, 0xbf),
                rng.range(0x80, 0xbf),
            ],
        );
    }
}

#[test]
fn config_10_valid_f4_boundary() {
    let pair = Pair::load();
    assert_valid(&pair, &[0xf4, 0x80, 0x80, 0x80]);
    assert_valid(&pair, &[0xf4, 0x8f, 0xbf, 0xbf]);
    let mut rng = Rng::new(0x1010_1010);
    for _ in 0..512 {
        assert_valid(
            &pair,
            &[
                0xf4,
                rng.range(0x80, 0x8f),
                rng.range(0x80, 0xbf),
                rng.range(0x80, 0xbf),
            ],
        );
    }
}

#[test]
fn config_11_mixed_valid_strings() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1111_1111);
    for _ in 0..256 {
        let mut bytes = Vec::new();
        for _ in 0..rng.usize(0, 128) {
            append_random_valid(&mut bytes, &mut rng);
        }
        assert_valid(&pair, &bytes);
    }
}

#[test]
fn config_12_valid_prefix_then_each_malformed_shape() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1212_1212);
    for (name, invalid) in invalid_shapes() {
        for _ in 0..64 {
            let mut bytes = Vec::new();
            for _ in 0..rng.usize(1, 24) {
                append_random_valid(&mut bytes, &mut rng);
            }
            let expected_offset = bytes.len();
            bytes.extend(&invalid);
            let result = pair.compare(&bytes);
            assert_eq!(
                result.drop_offset,
                expected_offset,
                "wrong C rejection offset for {name}: {}",
                hex(&bytes)
            );
        }
    }
}

fn random_malformed(pair: &Pair, replacement: bool, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..512 {
        let mut bytes = Vec::new();
        for _ in 0..rng.usize(1, 256) {
            bytes.push(rng.range(1, 0xff));
        }
        bytes.insert(rng.usize(0, bytes.len()), 0xff);
        let result = pair.compare(&bytes);
        assert!(result.drop_offset < bytes.len());
        let selected = if replacement {
            result.replace_output
        } else {
            result.delete_output
        };
        assert!(selected.len() <= bytes.len() * 3);
    }
}

#[test]
fn config_13_malformed_without_replacement() {
    random_malformed(&Pair::load(), false, 0x1313_1313);
}

#[test]
fn config_14_malformed_with_replacement() {
    random_malformed(&Pair::load(), true, 0x1414_1414);
}

fn alternating_input(rng: &mut Rng) -> Vec<u8> {
    let mut bytes = Vec::new();
    for _ in 0..rng.usize(2, 128) {
        append_random_valid(&mut bytes, rng);
        let shapes = invalid_shapes();
        let invalid = &shapes[rng.usize(0, shapes.len() - 1)].1;
        bytes.extend(invalid);
    }
    bytes
}

#[test]
fn config_15_alternating_without_replacement() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1515_1515);
    for _ in 0..128 {
        let bytes = alternating_input(&mut rng);
        let result = pair.compare(&bytes);
        assert!(result.drop_offset < bytes.len());
    }
}

#[test]
fn config_16_alternating_with_replacement() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1616_1616);
    for _ in 0..128 {
        let bytes = alternating_input(&mut rng);
        let result = pair.compare(&bytes);
        assert!(result.drop_offset < bytes.len());
    }
}

#[test]
fn config_17_multiple_replacement_growth_boundaries() {
    let pair = Pair::load();
    for length in [1366, 1367, 2731, 2732, 4096, 5000, 8192] {
        let bytes = vec![0xff; length];
        let result = pair.compare(&bytes);
        assert_eq!(result.drop_offset, 0);
        assert_eq!(result.delete_output, Vec::<u8>::new());
        assert_eq!(result.replace_output.len(), length * 3);
    }
}

#[test]
fn config_18_long_all_valid_inputs() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x1818_1818);
    for _ in 0..16 {
        let bytes = (0..65_536).map(|_| rng.range(1, 0x7f)).collect::<Vec<_>>();
        assert_valid(&pair, &bytes);
    }
}

#[test]
fn errors_03_through_16_exact_malformed_rejections() {
    let pair = Pair::load();
    let prefix = [b'A', 0xc2, 0x80];
    for (name, invalid) in invalid_shapes() {
        let mut bytes = prefix.to_vec();
        bytes.extend(invalid);
        let result = pair.compare(&bytes);
        assert_eq!(
            result.drop_offset,
            prefix.len(),
            "C did not reject {name} at the expected byte"
        );
    }
}

#[test]
fn null_pointer_child() {
    let Ok(case) = std::env::var("UTF8_DIFF_NULL_CASE") else {
        return;
    };
    let mut fields = case.split(':');
    let library = match fields.next().unwrap() {
        "c" => c_library(),
        "rust" => rust_library(),
        other => panic!("unknown library {other}"),
    };
    let api = unsafe { Api::load(&library) };
    match fields.next().unwrap() {
        "drop" => unsafe {
            (api.drop_fn)(std::ptr::null());
        },
        "filter" => unsafe {
            let pointer = (api.filter_fn)(std::ptr::null(), false);
            if !pointer.is_null() {
                free(pointer.cast());
            }
        },
        other => panic!("unknown symbol {other}"),
    }
    panic!("NULL call unexpectedly returned");
}

#[cfg(unix)]
#[test]
fn errors_01_02_null_assertions_match() {
    use std::os::unix::process::ExitStatusExt;

    let executable = std::env::current_exe().unwrap();
    for symbol in ["drop", "filter"] {
        let mut signals = Vec::new();
        for library in ["c", "rust"] {
            let status = Command::new(&executable)
                .args(["--exact", "null_pointer_child", "--nocapture"])
                .env("UTF8_DIFF_NULL_CASE", format!("{library}:{symbol}"))
                .status()
                .unwrap();
            signals.push(status.signal());
        }
        assert_eq!(signals[0], Some(6), "C {symbol} did not abort with SIGABRT");
        assert_eq!(
            signals[1], signals[0],
            "Rust and C reject NULL differently for {symbol}"
        );
    }
}

#[test]
fn allocation_failure_child() {
    if std::env::var_os("UTF8_DIFF_ALLOC_CHILD").is_none() {
        return;
    }

    type ArmFn = unsafe extern "C" fn();
    let process = libloading::os::unix::Library::this();
    let fail_strdup = unsafe { *process.get::<ArmFn>(b"fail_next_strdup\0").unwrap() };
    let fail_malloc = unsafe { *process.get::<ArmFn>(b"fail_next_malloc\0").unwrap() };
    let fail_realloc = unsafe { *process.get::<ArmFn>(b"fail_next_realloc\0").unwrap() };

    for library in [c_library(), rust_library()] {
        let api = unsafe { Api::load(&library) };

        let valid = b"valid\0";
        unsafe { fail_strdup() };
        let result = unsafe { (api.filter_fn)(valid.as_ptr().cast(), false) };
        assert!(
            result.is_null(),
            "{} did not propagate strdup failure",
            library.display()
        );

        let malformed = [0xff, 0];
        unsafe { fail_malloc() };
        let result = unsafe { (api.filter_fn)(malformed.as_ptr().cast(), false) };
        assert!(
            result.is_null(),
            "{} did not propagate malloc failure",
            library.display()
        );

        unsafe { fail_realloc() };
        let result = unsafe { (api.filter_fn)(malformed.as_ptr().cast(), true) };
        assert!(
            result.is_null(),
            "{} did not propagate realloc failure",
            library.display()
        );
    }
}

#[test]
fn errors_17_18_19_allocator_failures_return_null() {
    let helper = manifest_dir().join("target/fail_alloc.so");
    let status = Command::new("cc")
        .args([
            "-shared",
            "-fPIC",
            "-O2",
            "-o",
            helper.to_str().unwrap(),
            "tests/fail_alloc.c",
        ])
        .current_dir(manifest_dir())
        .status()
        .unwrap();
    assert!(status.success(), "failed to compile allocation shim");

    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "allocation_failure_child", "--nocapture"])
        .env("UTF8_DIFF_ALLOC_CHILD", "1")
        .env("LD_PRELOAD", helper)
        .status()
        .unwrap();
    assert!(status.success(), "allocator-failure child failed: {status}");
}
