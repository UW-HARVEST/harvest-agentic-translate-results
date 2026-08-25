use libloading::{Library, Symbol};
use std::ffi::{OsStr, c_char};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output};

type Bin2Hex = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;

const CHILD_BACKEND: &str = "BIN2HEX_CHILD_BACKEND";
const CHILD_SCENARIO: &str = "BIN2HEX_CHILD_SCENARIO";
const CHILD_BIN_LEN: &str = "BIN2HEX_CHILD_BIN_LEN";
const CHILD_HEX_LEN: &str = "BIN2HEX_CHILD_HEX_LEN";

#[derive(Clone, Copy)]
enum Backend {
    C,
    Rust,
}

impl Backend {
    fn env_value(self) -> &'static str {
        match self {
            Self::C => "c",
            Self::Rust => "rust",
        }
    }

    fn library_path(self) -> PathBuf {
        match self {
            Self::C => manifest_dir()
                .join("c_src")
                .join("build")
                .join("libtranslated_rust.so"),
            Self::Rust => rust_library_path(),
        }
    }
}

#[derive(Debug)]
struct CallResult {
    output: Vec<u8>,
    returned_base: bool,
}

struct FixedRng(u64);

impl FixedRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn usize_in(&mut self, start: usize, end_exclusive: usize) -> usize {
        start + self.next_u64() as usize % (end_exclusive - start)
    }

    fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("RUST_DYLIB_PATH") {
        return PathBuf::from(path);
    }

    let debug = manifest_dir()
        .join("target")
        .join("debug")
        .join("libbin2hex_lib.so");
    if debug.exists() {
        debug
    } else {
        manifest_dir()
            .join("target")
            .join("release")
            .join("libbin2hex_lib.so")
    }
}

unsafe fn load_library(path: &Path) -> Library {
    assert!(
        path.exists(),
        "shared library does not exist: {}",
        path.display()
    );
    unsafe { Library::new(path) }.unwrap_or_else(|error| {
        panic!("failed to load {}: {error}", path.display());
    })
}

unsafe fn load_bin2hex(library: &Library) -> Symbol<'_, Bin2Hex> {
    unsafe { library.get(b"bin2hex\0") }.expect("missing bin2hex export")
}

unsafe fn call(function: Bin2Hex, input: &[u8], capacity: usize, fill: u8) -> CallResult {
    let mut output = vec![fill; capacity];
    let base = output.as_mut_ptr();
    let returned =
        unsafe { function(base.cast::<c_char>(), capacity, input.as_ptr(), input.len()) };
    CallResult {
        output,
        returned_base: returned.cast::<u8>() == base,
    }
}

fn assert_case(input: &[u8], capacity: usize, fill: u8) {
    unsafe {
        let c_library = load_library(&Backend::C.library_path());
        let rust_library = load_library(&Backend::Rust.library_path());
        let c_function = load_bin2hex(&c_library);
        let rust_function = load_bin2hex(&rust_library);

        let c = call(*c_function, input, capacity, fill);
        let rust = call(*rust_function, input, capacity, fill);

        assert!(c.returned_base, "C did not return the output buffer");
        assert!(rust.returned_base, "Rust did not return the output buffer");
        assert_eq!(rust.output, c.output, "input: {input:02x?}");
    }
}

fn nibble(rng: &mut FixedRng, alphabetic: bool, boundary_case: usize) -> u8 {
    let (start, end): (usize, usize) = if alphabetic { (10, 16) } else { (0, 10) };
    match boundary_case % 3 {
        0 => start as u8,
        1 => (end - 1) as u8,
        _ => rng.usize_in(start, end) as u8,
    }
}

fn exercise_one_byte_class(seed: u64, high_alpha: bool, low_alpha: bool) {
    let mut rng = FixedRng::new(seed);
    for case in 0..256 {
        let high = nibble(&mut rng, high_alpha, case);
        let low = nibble(&mut rng, low_alpha, case + 1);
        assert_case(&[(high << 4) | low], 3, rng.byte());
    }
}

#[test]
fn config_01_empty_input() {
    let mut rng = FixedRng::new(0xe0_01_8a_7d);
    for _ in 0..256 {
        let capacity = rng.usize_in(1, 130);
        assert_case(&[], capacity, rng.byte());
    }
}

#[test]
fn config_02_one_numeric_numeric() {
    exercise_one_byte_class(0x02_90_09_00, false, false);
}

#[test]
fn config_03_one_numeric_alphabetic() {
    exercise_one_byte_class(0x03_90_0f_0a, false, true);
}

#[test]
fn config_04_one_alphabetic_numeric() {
    exercise_one_byte_class(0x04_f0_a0_90, true, false);
}

#[test]
fn config_05_one_alphabetic_alphabetic() {
    exercise_one_byte_class(0x05_ff_af_aa, true, true);
}

fn randomized_input(rng: &mut FixedRng, case: usize) -> Vec<u8> {
    if case == 0 {
        return (u8::MIN..=u8::MAX).collect();
    }

    let length = rng.usize_in(2, 2049);
    (0..length).map(|_| rng.byte()).collect()
}

#[test]
fn config_06_many_exact_capacity() {
    let mut rng = FixedRng::new(0x06_26_51_d4);
    for case in 0..256 {
        let input = randomized_input(&mut rng, case);
        assert_case(&input, input.len() * 2 + 1, rng.byte());
    }
}

#[test]
fn config_07_many_excess_capacity() {
    let mut rng = FixedRng::new(0x07_63_3c_9b);
    for case in 0..256 {
        let input = randomized_input(&mut rng, case);
        let extra = rng.usize_in(1, 129);
        assert_case(&input, input.len() * 2 + 1 + extra, rng.byte());
    }
}

fn child_command(backend: Backend, scenario: &str, bin_len: usize, hex_len: usize) -> Output {
    Command::new(std::env::current_exe().expect("current test executable"))
        .args(["--exact", "ffi_boundary_child", "--nocapture"])
        .env(CHILD_BACKEND, backend.env_value())
        .env(CHILD_SCENARIO, scenario)
        .env(CHILD_BIN_LEN, bin_len.to_string())
        .env(CHILD_HEX_LEN, hex_len.to_string())
        .output()
        .expect("run boundary child")
}

#[cfg(unix)]
fn signal(status: ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn signal(_status: ExitStatus) -> Option<i32> {
    None
}

fn assert_same_termination(
    scenario: &str,
    bin_len: usize,
    hex_len: usize,
    expected_signal: Option<i32>,
) {
    let c = child_command(Backend::C, scenario, bin_len, hex_len);
    let rust = child_command(Backend::Rust, scenario, bin_len, hex_len);

    assert!(!c.status.success(), "C unexpectedly returned successfully");
    assert!(
        !rust.status.success(),
        "Rust unexpectedly returned successfully"
    );
    assert_eq!(
        signal(rust.status),
        signal(c.status),
        "different termination for {scenario}; C stderr: {}; Rust stderr: {}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&rust.stderr),
    );
    if let Some(expected) = expected_signal {
        assert_eq!(signal(c.status), Some(expected));
    }
}

#[test]
fn error_01_oversized_length_aborts() {
    let threshold = usize::MAX / 2;
    for bin_len in [threshold, threshold + 1, usize::MAX - 1, usize::MAX] {
        assert_same_termination("guard", bin_len, usize::MAX, Some(6));
    }
}

#[test]
fn error_02_insufficient_capacity_aborts() {
    let cases = [
        (0, 0),
        (1, 0),
        (1, 1),
        (1, 2),
        (17, 33),
        (17, 34),
        (4096, 8192),
    ];
    for (bin_len, hex_len) in cases {
        assert_same_termination("guard", bin_len, hex_len, Some(6));
    }
}

#[test]
fn generic_null_output_pointer_terminates_identically() {
    assert_same_termination("null_hex", 0, 1, None);
}

#[test]
fn generic_null_input_pointer_terminates_identically() {
    assert_same_termination("null_bin", 1, 3, None);
}

#[test]
fn generic_null_input_is_valid_for_zero_length() {
    unsafe {
        let c_library = load_library(&Backend::C.library_path());
        let rust_library = load_library(&Backend::Rust.library_path());
        let c_function = load_bin2hex(&c_library);
        let rust_function = load_bin2hex(&rust_library);
        let mut c_output = [0x5a_u8];
        let mut rust_output = [0x5a_u8];

        let c_returned = c_function(
            c_output.as_mut_ptr().cast(),
            c_output.len(),
            std::ptr::null(),
            0,
        );
        let rust_returned = rust_function(
            rust_output.as_mut_ptr().cast(),
            rust_output.len(),
            std::ptr::null(),
            0,
        );

        assert_eq!(c_returned.cast::<u8>(), c_output.as_mut_ptr());
        assert_eq!(rust_returned.cast::<u8>(), rust_output.as_mut_ptr());
        assert_eq!(rust_output, c_output);
    }
}

#[test]
fn all_defined_c_symbols_are_loadable_from_rust() {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(Backend::C.library_path())
        .output()
        .expect("run nm");
    assert!(output.status.success(), "nm failed");

    unsafe {
        let rust_library = load_library(&Backend::Rust.library_path());
        for line in String::from_utf8(output.stdout)
            .expect("nm output is UTF-8")
            .lines()
        {
            let symbol = line.split_whitespace().last().expect("defined symbol name");
            let mut nul_terminated = symbol.as_bytes().to_vec();
            nul_terminated.push(0);
            let loaded = rust_library.get::<*const ()>(&nul_terminated);
            assert!(loaded.is_ok(), "Rust library is missing C symbol {symbol}");
        }
    }
}

#[test]
fn ffi_boundary_child() {
    let Some(backend) = std::env::var_os(CHILD_BACKEND) else {
        return;
    };
    let backend = match backend.as_os_str() {
        value if value == OsStr::new("c") => Backend::C,
        value if value == OsStr::new("rust") => Backend::Rust,
        value => panic!("unknown child backend: {value:?}"),
    };
    let scenario = std::env::var(CHILD_SCENARIO).expect("child scenario");
    let bin_len = std::env::var(CHILD_BIN_LEN)
        .expect("child bin length")
        .parse::<usize>()
        .expect("numeric child bin length");
    let hex_len = std::env::var(CHILD_HEX_LEN)
        .expect("child hex length")
        .parse::<usize>()
        .expect("numeric child hex length");

    unsafe {
        let library = load_library(&backend.library_path());
        let function = load_bin2hex(&library);
        match scenario.as_str() {
            "guard" => {
                function(std::ptr::null_mut(), hex_len, std::ptr::null(), bin_len);
            }
            "null_hex" => {
                function(std::ptr::null_mut(), hex_len, std::ptr::null(), bin_len);
            }
            "null_bin" => {
                let mut output = [0_u8; 3];
                function(
                    output.as_mut_ptr().cast(),
                    hex_len,
                    std::ptr::null(),
                    bin_len,
                );
            }
            value => panic!("unknown child scenario: {value}"),
        }
    }
}
