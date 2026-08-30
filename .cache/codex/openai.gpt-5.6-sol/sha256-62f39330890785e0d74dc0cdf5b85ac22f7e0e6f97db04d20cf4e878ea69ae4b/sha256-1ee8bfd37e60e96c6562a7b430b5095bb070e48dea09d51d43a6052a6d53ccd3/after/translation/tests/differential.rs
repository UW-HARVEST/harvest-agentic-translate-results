use libloading::Library;
use std::env;
use std::ffi::{c_char, c_int, c_void};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};

type Driver = unsafe extern "C" fn(*const c_char, *const c_char);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

const STDOUT_FILENO: c_int = 1;

struct Api {
    _library: Library,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let driver = unsafe {
            *library.get::<Driver>(b"driver\0").unwrap_or_else(|error| {
                panic!("failed to load driver from {}: {error}", path.display())
            })
        };
        Self {
            _library: library,
            driver,
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation crate must have a parent")
        .to_path_buf()
}

fn c_library_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let mut pipe_fds = [-1; 2];
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);

    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    call();

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    let mut output = Vec::new();
    let mut chunk = [0_u8; 128];
    loop {
        let count = unsafe { read(pipe_fds[0], chunk.as_mut_ptr().cast(), chunk.len()) };
        assert!(count >= 0);
        if count == 0 {
            break;
        }
        output.extend_from_slice(&chunk[..count as usize]);
    }
    assert_eq!(unsafe { close(pipe_fds[0]) }, 0);
    output
}

type Case = (Vec<u8>, Vec<u8>);

fn temporary_path(label: &str) -> PathBuf {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!("driver-diff-{}-{id}-{label}", std::process::id()))
}

fn encode_cases(cases: &[Case]) -> Vec<u8> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&(cases.len() as u64).to_le_bytes());
    for (s1, s2) in cases {
        encoded.extend_from_slice(&(s1.len() as u64).to_le_bytes());
        encoded.extend_from_slice(s1);
        encoded.extend_from_slice(&(s2.len() as u64).to_le_bytes());
        encoded.extend_from_slice(s2);
    }
    encoded
}

fn take_bytes<'a>(encoded: &mut &'a [u8], length: usize) -> &'a [u8] {
    assert!(encoded.len() >= length, "truncated encoded cases");
    let (value, remainder) = encoded.split_at(length);
    *encoded = remainder;
    value
}

fn take_u64(encoded: &mut &[u8]) -> u64 {
    u64::from_le_bytes(
        take_bytes(encoded, std::mem::size_of::<u64>())
            .try_into()
            .expect("u64 byte count"),
    )
}

fn decode_cases(mut encoded: &[u8]) -> Vec<Case> {
    let count = take_u64(&mut encoded) as usize;
    let cases = (0..count)
        .map(|_| {
            let s1_length = take_u64(&mut encoded) as usize;
            let s1 = take_bytes(&mut encoded, s1_length).to_vec();
            let s2_length = take_u64(&mut encoded) as usize;
            let s2 = take_bytes(&mut encoded, s2_length).to_vec();
            (s1, s2)
        })
        .collect();
    assert!(encoded.is_empty(), "extra bytes after encoded cases");
    cases
}

fn batch_status(library: &Path, input: &Path, output: &Path) -> ExitStatus {
    Command::new(env::current_exe().expect("current test executable"))
        .args(["--exact", "ffi_batch_child", "--nocapture"])
        .env("DRIVER_BATCH_LIBRARY", library)
        .env("DRIVER_BATCH_INPUT", input)
        .env("DRIVER_BATCH_OUTPUT", output)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("batch child process")
}

fn compare_cases(cases: &[Case]) {
    for (s1, s2) in cases {
        assert_eq!(s1.last(), Some(&0), "s1 must be NUL terminated");
        assert_eq!(s2.last(), Some(&0), "s2 must be NUL terminated");
    }

    let input_path = temporary_path("input");
    let c_output_path = temporary_path("c-output");
    let rust_output_path = temporary_path("rust-output");
    fs::write(&input_path, encode_cases(cases)).expect("write encoded cases");

    let c_status = batch_status(&c_library_path(), &input_path, &c_output_path);
    let rust_status = batch_status(&rust_library_path(), &input_path, &rust_output_path);
    assert!(c_status.success(), "C batch failed: {c_status:?}");
    assert!(rust_status.success(), "Rust batch failed: {rust_status:?}");

    let c_output = fs::read(&c_output_path).expect("read C output");
    let rust_output = fs::read(&rust_output_path).expect("read Rust output");
    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(c_output_path);
    let _ = fs::remove_file(rust_output_path);
    assert_eq!(rust_output, c_output, "batch output differs");
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

    fn range(&mut self, start: usize, end: usize) -> usize {
        assert!(start < end);
        start + self.next_u64() as usize % (end - start)
    }

    fn nonzero_byte(&mut self) -> u8 {
        self.range(1, 256) as u8
    }

    fn bytes(&mut self, length: usize) -> Vec<u8> {
        let mut bytes: Vec<u8> = (0..length).map(|_| self.nonzero_byte()).collect();
        bytes.push(0);
        bytes
    }
}

#[test]
fn config_1_empty_input() {
    let mut rng = Rng::new(0x0101_5eed);
    let mut cases = Vec::new();
    for _ in 0..128 {
        let length = rng.range(0, 128);
        let s2 = rng.bytes(length);
        cases.push((vec![0], s2));
    }
    compare_cases(&cases);
}

#[test]
fn config_2_empty_rejection_set() {
    let mut rng = Rng::new(0x0202_5eed);
    let mut cases = Vec::new();
    for _ in 0..128 {
        let length = rng.range(1, 257);
        let s1 = rng.bytes(length);
        cases.push((s1, vec![0]));
    }
    compare_cases(&cases);
}

#[test]
fn config_3_rejected_first_byte() {
    let mut rng = Rng::new(0x0303_5eed);
    let mut cases = Vec::new();
    for _ in 0..128 {
        let s1_length = rng.range(1, 257);
        let s2_length = rng.range(1, 65);
        let s1 = rng.bytes(s1_length);
        let mut s2 = rng.bytes(s2_length);
        s2.insert(rng.range(0, s2.len()), s1[0]);
        cases.push((s1, s2));
    }
    compare_cases(&cases);
}

#[test]
fn config_4_rejected_after_prefix() {
    let mut rng = Rng::new(0x0404_5eed);
    let mut cases = Vec::new();
    for _ in 0..128 {
        let rejected = rng.nonzero_byte();
        let prefix_length = rng.range(1, 257);
        let mut s1: Vec<u8> = (0..prefix_length)
            .map(|_| {
                let byte = rng.nonzero_byte();
                if byte == rejected {
                    rejected.wrapping_add(1).max(1)
                } else {
                    byte
                }
            })
            .collect();
        s1.push(rejected);
        s1.extend((0..rng.range(0, 65)).map(|_| rng.nonzero_byte()));
        s1.push(0);

        let mut s2 = vec![rejected; rng.range(1, 17)];
        s2.push(0);
        cases.push((s1, s2));
    }
    compare_cases(&cases);
}

#[test]
fn config_5_no_rejected_byte() {
    let mut rng = Rng::new(0x0505_5eed);
    let mut cases = Vec::new();
    for _ in 0..128 {
        let mut s1: Vec<u8> = (0..rng.range(1, 257))
            .map(|_| rng.range(1, 128) as u8)
            .collect();
        s1.push(0);
        let mut s2: Vec<u8> = (0..rng.range(1, 129))
            .map(|_| rng.range(128, 256) as u8)
            .collect();
        s2.push(0);
        cases.push((s1, s2));
    }
    compare_cases(&cases);
}

#[test]
fn config_6_bytes_after_nul_are_ignored() {
    let mut rng = Rng::new(0x0606_5eed);
    let mut cases = Vec::new();
    for _ in 0..128 {
        let visible_length = rng.range(1, 129);
        let mut s1 = rng.bytes(visible_length);
        let s1_trailing_length = rng.range(1, 129);
        s1.extend_from_slice(&rng.bytes(s1_trailing_length));

        let rejected_after_nul = s1[rng.range(0, visible_length)];
        let mut s2 = vec![rng.nonzero_byte(), 0, rejected_after_nul];
        let s2_trailing_length = rng.range(1, 65);
        s2.extend_from_slice(&rng.bytes(s2_trailing_length));
        cases.push((s1, s2));
    }
    compare_cases(&cases);
}

#[test]
fn config_7_non_ascii_bytes() {
    let mut rng = Rng::new(0x0707_5eed);
    let mut cases = Vec::new();
    for case in 0..128 {
        let mut s1: Vec<u8> = (0..rng.range(1, 257))
            .map(|_| rng.range(128, 256) as u8)
            .collect();
        s1.push(0);

        let mut s2: Vec<u8> = (0..rng.range(1, 65))
            .map(|_| rng.range(128, 256) as u8)
            .collect();
        if case % 2 == 0 {
            s2[0] = s1[rng.range(0, s1.len() - 1)];
        }
        s2.push(0);
        cases.push((s1, s2));
    }
    compare_cases(&cases);
}

#[test]
fn config_8_long_strings() {
    let mut rng = Rng::new(0x0808_5eed);
    let mut cases = Vec::new();
    for _ in 0..64 {
        let s1_length = rng.range(256, 8193);
        let s2_length = rng.range(1, 256);
        let s1 = rng.bytes(s1_length);
        let s2 = rng.bytes(s2_length);
        cases.push((s1, s2));
    }
    compare_cases(&cases);
}

#[test]
fn ffi_batch_child() {
    let Some(library_path) = env::var_os("DRIVER_BATCH_LIBRARY") else {
        return;
    };
    let input_path = env::var_os("DRIVER_BATCH_INPUT").expect("batch input path");
    let output_path = env::var_os("DRIVER_BATCH_OUTPUT").expect("batch output path");
    let cases = decode_cases(&fs::read(input_path).expect("read encoded cases"));
    let api = unsafe { Api::load(Path::new(&library_path)) };
    let output = unsafe {
        capture_stdout(|| {
            for (s1, s2) in &cases {
                (api.driver)(s1.as_ptr().cast(), s2.as_ptr().cast());
            }
        })
    };
    fs::write(output_path, output).expect("write captured output");
    std::process::exit(0);
}

fn child_status(library: &Path, null_argument: &str) -> ExitStatus {
    Command::new(env::current_exe().expect("current test executable"))
        .args(["--exact", "ffi_boundary_child", "--nocapture"])
        .env("DRIVER_BOUNDARY_LIBRARY", library)
        .env("DRIVER_NULL_ARGUMENT", null_argument)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("boundary child process")
}

fn compare_boundary(null_argument: &str) {
    let c_status = child_status(&c_library_path(), null_argument);
    let rust_status = child_status(&rust_library_path(), null_argument);
    assert_eq!(
        rust_status, c_status,
        "process results differ for null {null_argument}: C={c_status:?}, Rust={rust_status:?}"
    );
    assert!(
        !c_status.success(),
        "null {null_argument} unexpectedly succeeded"
    );
}

#[test]
fn boundary_1_null_s1() {
    compare_boundary("s1");
}

#[test]
fn boundary_2_null_s2() {
    compare_boundary("s2");
}

#[test]
fn ffi_boundary_child() {
    let Some(library_path) = env::var_os("DRIVER_BOUNDARY_LIBRARY") else {
        return;
    };
    let null_argument = env::var("DRIVER_NULL_ARGUMENT").expect("null argument selector");
    let api = unsafe { Api::load(Path::new(&library_path)) };
    let valid = b"valid\0";

    unsafe {
        match null_argument.as_str() {
            "s1" => (api.driver)(ptr::null(), valid.as_ptr().cast()),
            "s2" => (api.driver)(valid.as_ptr().cast(), ptr::null()),
            value => panic!("unknown null argument {value}"),
        }
    }
}
