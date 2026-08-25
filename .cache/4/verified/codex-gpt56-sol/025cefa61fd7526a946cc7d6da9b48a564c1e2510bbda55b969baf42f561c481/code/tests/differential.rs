use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs::{self, File};
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

const STDIN_FILENO: c_int = 0;
const STDOUT_FILENO: c_int = 1;
const RANDOM_CASES: usize = 128;
const MAIN_CASES: usize = 96;
const INVALID_CASES: usize = 24;

static NEXT_FILE_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, PartialEq, Eq)]
struct ProbeResult {
    returns: Vec<c_int>,
    stdout: Vec<u8>,
}

struct TempFiles {
    request: PathBuf,
    result: PathBuf,
}

impl TempFiles {
    fn new() -> Self {
        let id = NEXT_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let prefix = format!("driver-ffi-{}-{id}", std::process::id());
        let mut request = std::env::temp_dir();
        request.push(format!("{prefix}.request"));
        let mut result = std::env::temp_dir();
        result.push(format!("{prefix}.result"));
        Self { request, result }
    }
}

impl Drop for TempFiles {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.request);
        let _ = fs::remove_file(&self.result);
    }
}

fn c_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver_c.so")
}

fn rust_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so")
}

fn write_string_batch(path: &Path, strings: &[Vec<u8>]) {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(strings.len() as u32).to_le_bytes());
    for string in strings {
        bytes.extend_from_slice(&(string.len() as u32).to_le_bytes());
        bytes.extend_from_slice(string);
    }
    fs::write(path, bytes).unwrap();
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> u32 {
    let end = *cursor + 4;
    let value = u32::from_le_bytes(bytes[*cursor..end].try_into().unwrap());
    *cursor = end;
    value
}

fn read_string_batch(path: &Path) -> Vec<CString> {
    let bytes = fs::read(path).unwrap();
    let mut cursor = 0;
    let count = read_u32(&bytes, &mut cursor) as usize;
    let mut strings = Vec::with_capacity(count);
    for _ in 0..count {
        let length = read_u32(&bytes, &mut cursor) as usize;
        let end = cursor + length;
        strings.push(CString::new(&bytes[cursor..end]).unwrap());
        cursor = end;
    }
    assert_eq!(cursor, bytes.len());
    strings
}

fn write_probe_result(path: &Path, result: &ProbeResult) {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&(result.returns.len() as u32).to_le_bytes());
    for value in &result.returns {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&result.stdout);
    fs::write(path, bytes).unwrap();
}

fn read_probe_result(path: &Path) -> ProbeResult {
    let bytes = fs::read(path).unwrap();
    let mut cursor = 0;
    let count = read_u32(&bytes, &mut cursor) as usize;
    let mut returns = Vec::with_capacity(count);
    for _ in 0..count {
        let end = cursor + 4;
        returns.push(c_int::from_le_bytes(bytes[cursor..end].try_into().unwrap()));
        cursor = end;
    }
    ProbeResult {
        returns,
        stdout: bytes[cursor..].to_vec(),
    }
}

fn run_probe(
    library: &Path,
    operation: &str,
    count: usize,
    request: &[u8],
    strings: Option<&[Vec<u8>]>,
) -> ProbeResult {
    assert!(
        library.is_file(),
        "missing shared library: {}",
        library.display()
    );
    let files = TempFiles::new();
    if let Some(strings) = strings {
        write_string_batch(&files.request, strings);
    } else {
        fs::write(&files.request, request).unwrap();
    }

    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "subprocess_ffi_entry", "--nocapture"])
        .env("DRIVER_FFI_LIBRARY", library)
        .env("DRIVER_FFI_OPERATION", operation)
        .env("DRIVER_FFI_COUNT", count.to_string())
        .env("DRIVER_FFI_REQUEST", &files.request)
        .env("DRIVER_FFI_RESULT", &files.result)
        .status()
        .unwrap();
    assert!(status.success(), "FFI probe failed for {operation}");
    read_probe_result(&files.result)
}

fn compare_probe(
    operation: &str,
    count: usize,
    request: &[u8],
    strings: Option<&[Vec<u8>]>,
) -> ProbeResult {
    let c_result = run_probe(&c_library(), operation, count, request, strings);
    let rust_result = run_probe(&rust_library(), operation, count, request, strings);
    assert_eq!(
        c_result, rust_result,
        "{operation} diverged for request {request:?}"
    );
    c_result
}

fn next_random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn random_strings() -> Vec<Vec<u8>> {
    let mut state = 0x6a09_e667_f3bc_c909;
    let mut strings = vec![Vec::new(), vec![b'x']];
    while strings.len() < RANDOM_CASES {
        let length = (next_random(&mut state) % 257) as usize;
        let mut string = Vec::with_capacity(length);
        for _ in 0..length {
            let value = (next_random(&mut state) % 255 + 1) as u8;
            string.push(value);
        }
        strings.push(string);
    }
    strings
}

fn zero_input() -> Vec<u8> {
    let spellings = ["0", "+0", "-0", "00", "000000", " 0", "\t+000"];
    let mut input = String::new();
    for index in 0..MAIN_CASES {
        input.push_str(spellings[index % spellings.len()]);
        input.push('\n');
    }
    input.into_bytes()
}

fn nonzero_input() -> Vec<u8> {
    let mut state = 0xbb67_ae85_84ca_a73b;
    let mut input = String::new();
    for index in 0..MAIN_CASES {
        let mut value = next_random(&mut state) as i32;
        if value == 0 {
            value = 1;
        }
        match index % 3 {
            0 if value > 0 => input.push_str(&format!("+{value}\n")),
            1 => input.push_str(&format!("  {value}\n")),
            _ => input.push_str(&format!("{value}\t")),
        }
    }
    input.into_bytes()
}

fn invalid_inputs() -> Vec<Vec<u8>> {
    let mut state = 0x3c6e_f372_fe94_f82b;
    let mut inputs = vec![
        b"x\n".to_vec(),
        b"+\n".to_vec(),
        b"-\n".to_vec(),
        b" \t\n".to_vec(),
    ];
    while inputs.len() < INVALID_CASES {
        let length = (next_random(&mut state) % 31 + 1) as usize;
        let mut input = Vec::with_capacity(length + 1);
        for _ in 0..length {
            input.push(b'a' + (next_random(&mut state) % 26) as u8);
        }
        input.push(b'\n');
        inputs.push(input);
    }
    inputs
}

#[test]
fn differential_surface() {
    let strings = random_strings();
    let result = compare_probe("printLine", strings.len(), &[], Some(&strings));
    let expected: Vec<u8> = strings
        .iter()
        .flat_map(|string| string.iter().copied().chain([b'\n']))
        .collect();
    assert_eq!(result.stdout, expected);

    let bad_result = compare_probe("bad", RANDOM_CASES, &[], None);
    assert!(bad_result.stdout.is_empty());

    let good_result = compare_probe("good", RANDOM_CASES, &[], None);
    assert_eq!(
        good_result.stdout,
        b"helperGood1 string\n".repeat(RANDOM_CASES)
    );

    let zero = zero_input();
    let zero_result = compare_probe("main", MAIN_CASES, &zero, None);
    assert_eq!(zero_result.returns, vec![0; MAIN_CASES]);
    assert!(zero_result.stdout.is_empty());

    let nonzero = nonzero_input();
    let nonzero_result = compare_probe("main", MAIN_CASES, &nonzero, None);
    assert_eq!(nonzero_result.returns, vec![0; MAIN_CASES]);
    assert_eq!(
        nonzero_result.stdout,
        b"helperGood1 string\n".repeat(MAIN_CASES)
    );

    let integer_boundaries = b"2147483648\n-2147483649\n";
    let boundary_result = compare_probe("main", 2, integer_boundaries, None);
    assert_eq!(boundary_result.returns, vec![0; 2]);

    let eof_result = compare_probe("main", MAIN_CASES, &[], None);
    assert_eq!(eof_result.returns, vec![0; MAIN_CASES]);
    assert!(eof_result.stdout.is_empty());
    for invalid in invalid_inputs() {
        let invalid_result = compare_probe("main", 1, &invalid, None);
        assert_eq!(invalid_result.returns, vec![0]);
        assert!(invalid_result.stdout.is_empty());
    }

    let null_result = compare_probe("printLineNull", RANDOM_CASES, &[], None);
    assert!(null_result.stdout.is_empty());
}

#[test]
fn subprocess_ffi_entry() {
    let Ok(library_path) = std::env::var("DRIVER_FFI_LIBRARY") else {
        return;
    };
    let operation = std::env::var("DRIVER_FFI_OPERATION").unwrap();
    let count: usize = std::env::var("DRIVER_FFI_COUNT").unwrap().parse().unwrap();
    let request_path = PathBuf::from(std::env::var("DRIVER_FFI_REQUEST").unwrap());
    let result_path = PathBuf::from(std::env::var("DRIVER_FFI_RESULT").unwrap());

    let input = File::open(&request_path).unwrap();
    if operation == "main" {
        assert_eq!(
            unsafe { dup2(input.as_raw_fd(), STDIN_FILENO) },
            STDIN_FILENO
        );
    }

    let mut output_pipe = [0; 2];
    assert_eq!(unsafe { pipe(output_pipe.as_mut_ptr()) }, 0);
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0);
    unsafe {
        fflush(std::ptr::null_mut());
    }
    assert_eq!(
        unsafe { dup2(output_pipe[1], STDOUT_FILENO) },
        STDOUT_FILENO
    );
    assert_eq!(unsafe { close(output_pipe[1]) }, 0);

    let library = unsafe { Library::new(library_path) }.unwrap();
    let mut returns = Vec::new();
    unsafe {
        match operation.as_str() {
            "printLine" => {
                let function = library
                    .get::<unsafe extern "C" fn(*const c_char)>(b"printLine\0")
                    .unwrap();
                let strings = read_string_batch(&request_path);
                assert_eq!(strings.len(), count);
                for string in strings {
                    function(string.as_ptr());
                }
            }
            "printLineNull" => {
                let function = library
                    .get::<unsafe extern "C" fn(*const c_char)>(b"printLine\0")
                    .unwrap();
                for _ in 0..count {
                    function(std::ptr::null());
                }
            }
            "bad" | "good" => {
                let function = library
                    .get::<unsafe extern "C" fn()>(format!("{operation}\0").as_bytes())
                    .unwrap();
                for _ in 0..count {
                    function();
                }
            }
            "main" => {
                let function = library
                    .get::<unsafe extern "C" fn() -> c_int>(b"main\0")
                    .unwrap();
                for _ in 0..count {
                    returns.push(function());
                }
            }
            _ => panic!("unknown operation {operation}"),
        }
        fflush(std::ptr::null_mut());
    }

    assert_eq!(unsafe { dup2(saved_stdout, STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(saved_stdout) }, 0);
    let mut stdout = Vec::new();
    unsafe {
        File::from_raw_fd(output_pipe[0])
            .read_to_end(&mut stdout)
            .unwrap();
    }
    write_probe_result(&result_path, &ProbeResult { returns, stdout });
    std::process::exit(0);
}
