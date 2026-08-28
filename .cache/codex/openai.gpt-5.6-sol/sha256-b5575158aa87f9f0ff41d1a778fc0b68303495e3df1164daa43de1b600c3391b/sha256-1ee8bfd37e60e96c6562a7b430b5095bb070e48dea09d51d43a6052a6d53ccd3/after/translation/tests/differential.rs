use libloading::{Library, Symbol};
use std::env;
use std::ffi::{CStr, c_char};
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

type ToolBasename = unsafe extern "C" fn(*mut c_char) -> *mut c_char;

const RANDOM_CASES_PER_ROW: usize = 512;
const CHILD_LIBRARY_ENV: &str = "DRIVER_NULL_TEST_LIBRARY";

#[derive(Debug, PartialEq)]
struct Outcome {
    offset: usize,
    suffix: Vec<u8>,
    buffer: Vec<u8>,
}

struct Rng(u64);

impl Rng {
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

    fn usize(&mut self, upper_exclusive: usize) -> usize {
        (self.next_u64() as usize) % upper_exclusive
    }

    fn non_separator(&mut self) -> u8 {
        loop {
            let byte = (self.next_u64() as u8).wrapping_add(1);
            if byte != b'/' && byte != b'\\' && byte != 0 {
                return byte;
            }
        }
    }
}

fn c_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

unsafe fn invoke(library: &Library, input: &[u8]) -> Outcome {
    let function: Symbol<ToolBasename> =
        unsafe { library.get(b"tool_basename\0") }.expect("load tool_basename");
    let mut buffer = input.to_vec();
    buffer.push(0);
    let original = buffer.clone();
    let start = buffer.as_mut_ptr().cast::<c_char>();
    let returned = unsafe { function(start) };

    assert!(!returned.is_null(), "valid input returned NULL");
    let offset = unsafe { returned.offset_from(start) };
    assert!(
        (0..buffer.len() as isize).contains(&offset),
        "returned pointer is outside the input buffer"
    );
    let suffix = unsafe { CStr::from_ptr(returned) }
        .to_bytes_with_nul()
        .to_vec();
    assert_eq!(buffer, original, "tool_basename modified its input");

    Outcome {
        offset: offset as usize,
        suffix,
        buffer,
    }
}

fn compare_cases(cases: impl IntoIterator<Item = Vec<u8>>) {
    let c_library = unsafe { Library::new(c_library_path()) }.expect("load C library");
    let rust_library = unsafe { Library::new(rust_library_path()) }.expect("load Rust library");

    for input in cases {
        let c_outcome = unsafe { invoke(&c_library, &input) };
        let rust_outcome = unsafe { invoke(&rust_library, &input) };
        assert_eq!(
            rust_outcome, c_outcome,
            "C/Rust mismatch for input bytes {input:?}"
        );
    }
}

fn random_non_separator_bytes(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.non_separator()).collect()
}

#[test]
fn config_1_no_separators() {
    let mut rng = Rng::new(0x6f2c_51d8_d153_2c77);
    let mut cases = vec![Vec::new(), vec![b'a'], vec![0xff; 256]];
    cases.extend((0..RANDOM_CASES_PER_ROW).map(|_| {
        let len = rng.usize(257);
        random_non_separator_bytes(&mut rng, len)
    }));
    compare_cases(cases);
}

#[test]
fn config_2_slashes_only() {
    let mut rng = Rng::new(0x8c01_a2fe_e882_56b1);
    let mut cases = vec![vec![b'/'], b"/a".to_vec(), b"a/".to_vec(), b"a//b".to_vec()];
    cases.extend((0..RANDOM_CASES_PER_ROW).map(|_| {
        let len = 1 + rng.usize(256);
        let mut input = random_non_separator_bytes(&mut rng, len);
        let separator_count = 1 + rng.usize(len.min(16));
        for _ in 0..separator_count {
            let index = rng.usize(len);
            input[index] = b'/';
        }
        input
    }));
    compare_cases(cases);
}

#[test]
fn config_3_backslashes_only() {
    let mut rng = Rng::new(0xd02e_9537_4381_9aa5);
    let mut cases = vec![
        vec![b'\\'],
        b"\\a".to_vec(),
        b"a\\".to_vec(),
        b"a\\\\b".to_vec(),
    ];
    cases.extend((0..RANDOM_CASES_PER_ROW).map(|_| {
        let len = 1 + rng.usize(256);
        let mut input = random_non_separator_bytes(&mut rng, len);
        let separator_count = 1 + rng.usize(len.min(16));
        for _ in 0..separator_count {
            let index = rng.usize(len);
            input[index] = b'\\';
        }
        input
    }));
    compare_cases(cases);
}

fn random_both_separators(rng: &mut Rng, last_separator: u8, earlier_separator: u8) -> Vec<u8> {
    let len = 2 + rng.usize(255);
    let last_index = 1 + rng.usize(len - 1);
    let earlier_index = rng.usize(last_index);
    let mut input = random_non_separator_bytes(rng, len);

    for byte in &mut input[..last_index] {
        if rng.usize(5) == 0 {
            *byte = if rng.usize(2) == 0 { b'/' } else { b'\\' };
        }
    }
    input[earlier_index] = earlier_separator;
    input[last_index] = last_separator;
    input
}

#[test]
fn config_4_both_with_slash_last() {
    let mut rng = Rng::new(0xa188_8965_e5c0_105f);
    let mut cases = vec![b"\\/".to_vec(), b"a\\b/c".to_vec(), b"\\\\//".to_vec()];
    cases.extend((0..RANDOM_CASES_PER_ROW).map(|_| random_both_separators(&mut rng, b'/', b'\\')));
    compare_cases(cases);
}

#[test]
fn config_5_both_with_backslash_last() {
    let mut rng = Rng::new(0x412e_20fb_ef0a_dd5b);
    let mut cases = vec![b"/\\".to_vec(), b"a/b\\c".to_vec(), b"//\\\\".to_vec()];
    cases.extend((0..RANDOM_CASES_PER_ROW).map(|_| random_both_separators(&mut rng, b'\\', b'/')));
    compare_cases(cases);
}

#[test]
fn ffi_null_pointer_child() {
    let Some(path) = env::var_os(CHILD_LIBRARY_ENV) else {
        return;
    };
    let library = unsafe { Library::new(path) }.expect("load child test library");
    let function: Symbol<ToolBasename> =
        unsafe { library.get(b"tool_basename\0") }.expect("load tool_basename");
    unsafe {
        function(std::ptr::null_mut());
    }
}

fn null_child_status(library: PathBuf) -> ExitStatus {
    Command::new(env::current_exe().expect("locate integration test executable"))
        .args(["--exact", "ffi_null_pointer_child", "--nocapture"])
        .env(CHILD_LIBRARY_ENV, library)
        .status()
        .expect("run isolated null-pointer child")
}

#[cfg(unix)]
#[test]
fn null_pointer_boundary_matches() {
    use std::os::unix::process::ExitStatusExt;

    let c_status = null_child_status(c_library_path());
    let rust_status = null_child_status(rust_library_path());
    assert!(!c_status.success(), "C unexpectedly accepted NULL");
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "C and Rust terminated with different signals"
    );
}
