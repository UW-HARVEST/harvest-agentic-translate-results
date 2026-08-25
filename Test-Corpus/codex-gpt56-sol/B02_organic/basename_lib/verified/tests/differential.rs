use libloading::Library;
use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type ToolBasename = unsafe extern "C" fn(*mut c_char) -> *mut c_char;

struct Api {
    _library: Library,
    tool_basename: ToolBasename,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let tool_basename = unsafe {
            *library
                .get::<ToolBasename>(b"tool_basename\0")
                .unwrap_or_else(|error| {
                    panic!(
                        "failed to load tool_basename from {}: {error}",
                        path.display()
                    )
                })
        };
        Self {
            _library: library,
            tool_basename,
        }
    }

    unsafe fn call(&self, input: &[u8]) -> Outcome {
        let first_nul = input
            .iter()
            .position(|byte| *byte == 0)
            .expect("test input must contain a NUL terminator");
        let original = input.to_vec();
        let mut buffer = original.clone();
        let base = buffer.as_mut_ptr();
        let returned = unsafe { (self.tool_basename)(base.cast::<c_char>()) }.cast::<u8>();

        let base_address = base as usize;
        let returned_address = returned as usize;
        assert!(
            (base_address..=base_address + first_nul).contains(&returned_address),
            "returned pointer {returned_address:#x} is outside the logical C string \
             {base_address:#x}..={:#x}",
            base_address + first_nul
        );
        let offset = returned_address - base_address;

        Outcome {
            offset,
            suffix: buffer[offset..=first_nul].to_vec(),
            unchanged: buffer == original,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    offset: usize,
    suffix: Vec<u8>,
    unchanged: bool,
}

struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        assert_ne!(seed, 0);
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

    fn usize(&mut self, upper_exclusive: usize) -> usize {
        (self.next_u64() as usize) % upper_exclusive
    }

    fn nonzero_byte(&mut self) -> u8 {
        (self.usize(255) + 1) as u8
    }

    fn ordinary_byte(&mut self) -> u8 {
        loop {
            let byte = self.nonzero_byte();
            if byte != b'/' && byte != b'\\' {
                return byte;
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    std::env::current_exe()
        .expect("integration test executable path")
        .parent()
        .expect("integration test deps directory")
        .join("libdriver.so")
}

fn load_apis() -> (Api, Api) {
    unsafe {
        (
            Api::load(&c_library_path()),
            Api::load(&rust_library_path()),
        )
    }
}

fn assert_parity(c_api: &Api, rust_api: &Api, cases: impl IntoIterator<Item = Vec<u8>>) {
    for (case_index, input) in cases.into_iter().enumerate() {
        let c_outcome = unsafe { c_api.call(&input) };
        let rust_outcome = unsafe { rust_api.call(&input) };
        assert_eq!(
            rust_outcome, c_outcome,
            "differential mismatch for case {case_index}, input {input:?}"
        );
        assert!(
            c_outcome.unchanged,
            "C unexpectedly changed case {case_index}, input {input:?}"
        );
        assert!(
            rust_outcome.unchanged,
            "Rust unexpectedly changed case {case_index}, input {input:?}"
        );
    }
}

fn random_ordinary_string(rng: &mut XorShift64, length: usize) -> Vec<u8> {
    (0..length).map(|_| rng.ordinary_byte()).collect()
}

#[test]
fn config_row_1_empty_string() {
    let (c_api, rust_api) = load_apis();
    let mut rng = XorShift64::new(0x3c79_ac49_2ba7_b653);
    let cases = (0..256).map(|_| {
        let trailing_length = rng.usize(128);
        let mut input = Vec::with_capacity(trailing_length + 1);
        input.push(0);
        input.extend((0..trailing_length).map(|_| rng.nonzero_byte()));
        input
    });
    assert_parity(&c_api, &rust_api, cases);
}

#[test]
fn config_row_2_nonempty_without_separators() {
    let (c_api, rust_api) = load_apis();
    let mut rng = XorShift64::new(0x1c69_b3f7_4ac4_ae35);
    let cases = (0..512).map(|_| {
        let length = rng.usize(256) + 1;
        let mut input = random_ordinary_string(&mut rng, length);
        input.push(0);
        input
    });
    assert_parity(&c_api, &rust_api, cases);
}

fn one_separator_cases(seed: u64, separator: u8) -> Vec<Vec<u8>> {
    let mut cases = vec![
        vec![separator, 0],
        vec![separator, b'a', 0],
        vec![b'a', separator, 0],
        vec![b'a', separator, b'b', separator, b'c', 0],
    ];
    let mut rng = XorShift64::new(seed);
    for _ in 0..512 {
        let length = rng.usize(256) + 1;
        let mut input = random_ordinary_string(&mut rng, length);
        let separator_count = rng.usize(length.min(16)) + 1;
        for _ in 0..separator_count {
            let position = rng.usize(length);
            input[position] = separator;
        }
        input.push(0);
        cases.push(input);
    }
    cases
}

#[test]
fn config_row_3_forward_slashes_only() {
    let (c_api, rust_api) = load_apis();
    assert_parity(
        &c_api,
        &rust_api,
        one_separator_cases(0x3d5d_5b75_61bf_42e9, b'/'),
    );
}

#[test]
fn config_row_4_backslashes_only() {
    let (c_api, rust_api) = load_apis();
    assert_parity(
        &c_api,
        &rust_api,
        one_separator_cases(0x4a6c_2d8b_7495_a153, b'\\'),
    );
}

fn both_separator_cases(seed: u64, last_separator: u8) -> Vec<Vec<u8>> {
    let earlier_separator = if last_separator == b'/' { b'\\' } else { b'/' };
    let mut rng = XorShift64::new(seed);
    let mut cases = Vec::with_capacity(512);
    for _ in 0..512 {
        let length = rng.usize(255) + 2;
        let mut input = random_ordinary_string(&mut rng, length);
        let last_position = rng.usize(length - 1) + 1;
        let earlier_position = rng.usize(last_position);

        for _ in 0..rng.usize(16) {
            let position = rng.usize(last_position);
            input[position] = if rng.usize(2) == 0 { b'/' } else { b'\\' };
        }
        input[earlier_position] = earlier_separator;
        input[last_position] = last_separator;
        input.push(0);
        cases.push(input);
    }
    cases
}

#[test]
fn config_row_5_last_forward_slash_after_last_backslash() {
    let (c_api, rust_api) = load_apis();
    assert_parity(
        &c_api,
        &rust_api,
        both_separator_cases(0x6f2a_1b93_58e4_7dc5, b'/'),
    );
}

#[test]
fn config_row_6_last_backslash_after_last_forward_slash() {
    let (c_api, rust_api) = load_apis();
    assert_parity(
        &c_api,
        &rust_api,
        both_separator_cases(0x7894_c32d_17ab_5ef1, b'\\'),
    );
}

#[test]
fn config_row_7_bytes_after_first_nul_are_ignored() {
    let (c_api, rust_api) = load_apis();
    let mut rng = XorShift64::new(0x7a1f_9c43_2db5_68e7);
    let cases = (0..512).map(|_| {
        let prefix_length = rng.usize(256);
        let trailing_length = rng.usize(256) + 1;
        let mut input: Vec<u8> = (0..prefix_length).map(|_| rng.nonzero_byte()).collect();
        input.push(0);
        input.extend((0..trailing_length).map(|_| rng.nonzero_byte()));
        input
    });
    assert_parity(&c_api, &rust_api, cases);
}

fn run_null_child(library: &Path) -> ExitStatus {
    Command::new(std::env::current_exe().expect("integration test executable path"))
        .args(["--exact", "null_pointer_child", "--nocapture"])
        .env("TOOL_BASENAME_NULL_LIBRARY", library)
        .status()
        .unwrap_or_else(|error| {
            panic!(
                "failed to run null-pointer child for {}: {error}",
                library.display()
            )
        })
}

#[test]
fn error_row_g1_null_pointer() {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;

        let c_status = run_null_child(&c_library_path());
        let rust_status = run_null_child(&rust_library_path());
        assert_eq!(
            rust_status.signal(),
            c_status.signal(),
            "null-pointer termination signal differs: C={c_status:?}, Rust={rust_status:?}"
        );
        assert!(
            c_status.signal().is_some(),
            "C unexpectedly returned normally for a null path: {c_status:?}"
        );
    }
}

#[test]
fn null_pointer_child() {
    let Ok(library_path) = std::env::var("TOOL_BASENAME_NULL_LIBRARY") else {
        return;
    };

    let api = unsafe { Api::load(Path::new(&library_path)) };
    unsafe {
        (api.tool_basename)(std::ptr::null_mut());
    }
}
