use libloading::Library;
use std::ffi::{c_char, c_double, c_int, c_void, CString};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

type DriverFn = unsafe extern "C" fn(c_double);
type MainFn = unsafe extern "C" fn() -> c_int;

unsafe extern "C" {
    static mut stdin: *mut c_void;
    static mut stdout: *mut c_void;

    fn fclose(stream: *mut c_void) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn free(pointer: *mut c_void);
    fn freopen(path: *const c_char, mode: *const c_char, stream: *mut c_void) -> *mut c_void;
    fn open_memstream(buffer: *mut *mut c_char, size: *mut usize) -> *mut c_void;
}

struct Api {
    _library: Library,
    driver: DriverFn,
    main: MainFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let driver = unsafe { *library.get::<DriverFn>(b"driver\0").unwrap() };
        let main = unsafe { *library.get::<MainFn>(b"main\0").unwrap() };

        Self {
            _library: library,
            driver,
            main,
        }
    }
}

struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        assert_ne!(seed, 0);
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

    fn below(&mut self, limit: u64) -> u64 {
        self.next() % limit
    }
}

static PROCESS_IO: Mutex<()> = Mutex::new(());
static INPUT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn libraries() -> (Api, Api) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = manifest.join("c_src/build/libdriver_c.so");
    let rust_path = std::env::current_exe()
        .expect("test executable path")
        .parent()
        .expect("test executable directory")
        .join("libdriver.so");

    assert!(
        c_path.is_file(),
        "missing C shared library: {}",
        c_path.display()
    );
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {}",
        rust_path.display()
    );

    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn capture_stdout<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    unsafe {
        let mut buffer = ptr::null_mut();
        let mut length = 0;
        let stream = open_memstream(&mut buffer, &mut length);
        assert!(!stream.is_null());
        let saved_stdout = stdout;
        stdout = stream;

        let result = call();

        assert_eq!(fflush(stream), 0);
        stdout = saved_stdout;
        assert_eq!(fclose(stream), 0);

        let output = slice::from_raw_parts(buffer.cast::<u8>(), length).to_vec();
        free(buffer.cast());
        (result, output)
    }
}

fn run_driver(api: &Api, bits: u64) -> Vec<u8> {
    let (_, output) = capture_stdout(|| unsafe { (api.driver)(f64::from_bits(bits)) });
    output
}

fn input_path() -> PathBuf {
    let sequence = INPUT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver-differential-{}-{sequence}.txt",
        std::process::id()
    ))
}

fn run_main(api: &Api, input: &[u8]) -> (c_int, Vec<u8>) {
    let path = input_path();
    fs::write(&path, input).expect("write temporary stdin");
    let c_path = CString::new(path.as_os_str().as_bytes()).expect("stdin path contains NUL");

    unsafe {
        assert!(!freopen(c_path.as_ptr(), c"r".as_ptr(), stdin).is_null());
    }
    let result = capture_stdout(|| unsafe { (api.main)() });
    fs::remove_file(path).expect("remove temporary stdin");
    result
}

fn assert_driver_cases(row: u8, cases: &[u64]) {
    let _io = PROCESS_IO.lock().unwrap();
    let (c_api, rust_api) = libraries();

    for &bits in cases {
        let c_output = run_driver(&c_api, bits);
        let rust_output = run_driver(&rust_api, bits);
        assert_eq!(
            rust_output, c_output,
            "CONFIGS.md row {row}, double bits 0x{bits:016x}"
        );
    }
}

fn assert_main_cases(row: u8, cases: &[Vec<u8>]) {
    let _io = PROCESS_IO.lock().unwrap();
    let (c_api, rust_api) = libraries();

    for input in cases {
        let c_result = run_main(&c_api, input);
        let rust_result = run_main(&rust_api, input);
        assert_eq!(
            rust_result,
            c_result,
            "CONFIGS.md row {row}, stdin {:?}",
            String::from_utf8_lossy(input)
        );
    }
}

fn random_ascii_case(rng: &mut XorShift64, text: &str) -> String {
    text.bytes()
        .map(|byte| {
            if rng.next() & 1 == 0 {
                byte.to_ascii_lowercase()
            } else {
                byte.to_ascii_uppercase()
            }
        })
        .map(char::from)
        .collect()
}

#[test]
fn config_01_driver_signed_zero() {
    assert_driver_cases(1, &[0, 1_u64 << 63]);
}

#[test]
fn config_02_driver_subnormals() {
    let mut rng = XorShift64::new(0x02d1_6f4b_9a75_c301);
    let mut cases = vec![1, (1_u64 << 52) - 1, (1_u64 << 63) | 1];
    while cases.len() < 128 {
        let mantissa = (rng.next() & ((1_u64 << 52) - 1)).max(1);
        cases.push((rng.next() & (1_u64 << 63)) | mantissa);
    }
    assert_driver_cases(2, &cases);
}

#[test]
fn config_03_driver_normals() {
    let mut rng = XorShift64::new(0x03a4_d8c2_17be_690f);
    let mut cases = vec![
        1_u64 << 52,
        0x7fef_ffff_ffff_ffff,
        (1_u64 << 63) | (1_u64 << 52),
        0xffef_ffff_ffff_ffff,
    ];
    while cases.len() < 128 {
        let sign = rng.next() & (1_u64 << 63);
        let exponent = 1 + rng.below(2046);
        let mantissa = rng.next() & ((1_u64 << 52) - 1);
        cases.push(sign | (exponent << 52) | mantissa);
    }
    assert_driver_cases(3, &cases);
}

#[test]
fn config_04_driver_infinities() {
    assert_driver_cases(4, &[0x7ff0_0000_0000_0000, 0xfff0_0000_0000_0000]);
}

#[test]
fn config_05_driver_nans() {
    let mut rng = XorShift64::new(0x05b7_23e9_c84a_1d60);
    let mut cases = vec![
        0x7ff8_0000_0000_0000,
        0xfff8_0000_0000_0000,
        0x7ff0_0000_0000_0001,
        0xfff0_0000_0000_0001,
    ];
    while cases.len() < 128 {
        let sign = rng.next() & (1_u64 << 63);
        let mut payload = rng.next() & ((1_u64 << 52) - 1);
        payload = payload.max(1);
        if rng.next() & 1 == 0 {
            payload |= 1_u64 << 51;
        } else {
            payload &= !(1_u64 << 51);
            payload = payload.max(1);
        }
        cases.push(sign | (0x7ff_u64 << 52) | payload);
    }
    assert_driver_cases(5, &cases);
}

#[test]
fn config_06_main_decimal_normal_and_zero() {
    let mut rng = XorShift64::new(0x061c_f4a8_92e3_5b70);
    let mut cases = vec![
        b"0".to_vec(),
        b"-0".to_vec(),
        b"+0.0000".to_vec(),
        b"1.7976931348623157e308".to_vec(),
        b"2.2250738585072014e-308".to_vec(),
    ];
    while cases.len() < 96 {
        let sign = if rng.next() & 1 == 0 { "" } else { "-" };
        let whole = 1 + rng.below(100_000);
        let fraction = rng.below(1_000_000_000);
        let exponent = rng.below(401) as i32 - 200;
        cases.push(format!("{sign}{whole}.{fraction:09}e{exponent:+}").into_bytes());
    }
    assert_main_cases(6, &cases);
}

#[test]
fn config_07_main_decimal_subnormal_and_signed_zero() {
    let mut rng = XorShift64::new(0x074e_1ab9_d360_82c5);
    let mut cases = vec![
        b"4.9406564584124654e-324".to_vec(),
        b"-4.9406564584124654e-324".to_vec(),
        b"2.225073858507201e-308".to_vec(),
        b"-0e999".to_vec(),
    ];
    while cases.len() < 96 {
        let sign = if rng.next() & 1 == 0 { "" } else { "-" };
        let digit = 1 + rng.below(9);
        let exponent = 309 + rng.below(15);
        cases.push(format!("{sign}{digit}e-{exponent}").into_bytes());
    }
    assert_main_cases(7, &cases);
}

#[test]
fn config_08_main_hexadecimal_finite() {
    let mut rng = XorShift64::new(0x08e2_759c_41bf_a630);
    let mut cases = vec![
        b"0x1p0".to_vec(),
        b"-0x1.fffffffffffffp1023".to_vec(),
        b"0x0.0000000000001p-1022".to_vec(),
    ];
    while cases.len() < 96 {
        let sign = if rng.next() & 1 == 0 { "" } else { "-" };
        if rng.next() & 1 == 0 {
            let fraction = rng.next() & ((1_u64 << 52) - 1);
            let exponent = rng.below(2001) as i32 - 1000;
            cases.push(format!("{sign}0x1.{fraction:013x}p{exponent:+}").into_bytes());
        } else {
            let fraction = (rng.next() & ((1_u64 << 52) - 1)).max(1);
            cases.push(format!("{sign}0x0.{fraction:013x}p-1022").into_bytes());
        }
    }
    assert_main_cases(8, &cases);
}

#[test]
fn config_09_main_infinity_spellings() {
    let mut rng = XorShift64::new(0x093d_b168_7c2f_e450);
    let mut cases = Vec::new();
    while cases.len() < 96 {
        let sign = match rng.below(3) {
            0 => "",
            1 => "+",
            _ => "-",
        };
        let word = if rng.next() & 1 == 0 {
            random_ascii_case(&mut rng, "inf")
        } else {
            random_ascii_case(&mut rng, "infinity")
        };
        cases.push(format!("{sign}{word}").into_bytes());
    }
    assert_main_cases(9, &cases);
}

#[test]
fn config_10_main_nan_spellings() {
    let mut rng = XorShift64::new(0x10fa_4c82_b715_3e69);
    let mut cases = Vec::new();
    while cases.len() < 96 {
        let sign = match rng.below(3) {
            0 => "",
            1 => "+",
            _ => "-",
        };
        let word = random_ascii_case(&mut rng, "nan");
        if rng.next() & 1 == 0 {
            cases.push(format!("{sign}{word}").into_bytes());
        } else {
            cases.push(format!("{sign}{word}(0x{:x})", rng.next()).into_bytes());
        }
    }
    assert_main_cases(10, &cases);
}

#[test]
fn config_11_main_decimal_overflow() {
    let mut rng = XorShift64::new(0x11c8_30e7_5a4d_92b6);
    let mut cases = Vec::new();
    while cases.len() < 96 {
        let sign = if rng.next() & 1 == 0 { "" } else { "-" };
        let significand = 1 + rng.below(10_000_000);
        let exponent = 309 + rng.below(10_000);
        cases.push(format!("{sign}{significand}e{exponent}").into_bytes());
    }
    assert_main_cases(11, &cases);
}

#[test]
fn config_12_main_decimal_underflow() {
    let mut rng = XorShift64::new(0x12a6_f359_08de_47c1);
    let mut cases = Vec::new();
    while cases.len() < 96 {
        let sign = if rng.next() & 1 == 0 { "" } else { "-" };
        let significand = 1 + rng.below(10_000_000);
        let exponent = 324 + rng.below(10_000);
        cases.push(format!("{sign}{significand}e-{exponent}").into_bytes());
    }
    assert_main_cases(12, &cases);
}

#[test]
fn config_13_main_whitespace_and_trailing_bytes() {
    let mut rng = XorShift64::new(0x13d9_674a_2be1_805c);
    let prefixes = [" ", "\t", "\n", " \t\n", "\n\n\t"];
    let suffixes = ["xyz", ",next", "#comment", " trailing", "\nrest"];
    let mut cases = Vec::new();
    while cases.len() < 96 {
        let prefix = prefixes[rng.below(prefixes.len() as u64) as usize];
        let suffix = suffixes[rng.below(suffixes.len() as u64) as usize];
        let sign = if rng.next() & 1 == 0 { "" } else { "-" };
        let whole = 1 + rng.below(1_000_000);
        let fraction = rng.below(1_000_000);
        cases.push(format!("{prefix}{sign}{whole}.{fraction:06}{suffix}").into_bytes());
    }
    assert_main_cases(13, &cases);
}

#[test]
fn config_14_main_conversion_failure() {
    let mut rng = XorShift64::new(0x14b2_0c68_f53a_97de);
    let prefixes = ["x", "@", "--", "..", "e", "_", "not-a-number"];
    let mut cases = Vec::new();
    while cases.len() < 96 {
        let prefix = prefixes[rng.below(prefixes.len() as u64) as usize];
        cases.push(format!("{prefix}{:x}", rng.next()).into_bytes());
    }
    assert_main_cases(14, &cases);
}

#[test]
fn config_15_main_empty_stdin() {
    let mut rng = XorShift64::new(0x15e7_49bd_31c6_a280);
    let mut cases = vec![Vec::new()];
    while cases.len() < 96 {
        let length = rng.below(64) as usize;
        let input = (0..length)
            .map(|_| match rng.below(3) {
                0 => b' ',
                1 => b'\t',
                _ => b'\n',
            })
            .collect();
        cases.push(input);
    }
    assert_main_cases(15, &cases);
}
