use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

type FileHandle = c_void;
type VoidFn = unsafe extern "C" fn();
type PrintLineFn = unsafe extern "C" fn(*const c_char);
type PrintIntLineFn = unsafe extern "C" fn(c_int);
type MainFn = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

unsafe extern "C" {
    static mut stdin: *mut FileHandle;

    fn __fpurge(stream: *mut FileHandle);
    fn clearerr(stream: *mut FileHandle);
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut FileHandle) -> c_int;
}

static STDIO_LOCK: Mutex<()> = Mutex::new(());
static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);
static RUST_LIBRARY: OnceLock<PathBuf> = OnceLock::new();

#[derive(Clone)]
enum Input {
    Bytes(Vec<u8>),
    ReadError,
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as u32
    }

    fn index(&mut self) -> u8 {
        (self.next_u32() % 10) as u8
    }
}

fn c_library_path() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let library = root.join("c_src/build/libdriver_c.so");
    if !library.exists() {
        fs::create_dir_all(root.join("c_src/build")).expect("create C build directory");
        let status = Command::new("cc")
            .args(["-shared", "-fPIC", "-o"])
            .arg(&library)
            .arg(root.join("c_src/src/main.c"))
            .status()
            .expect("invoke C compiler");
        assert!(status.success(), "failed to build C shared library");
    }
    library
}

fn rust_library_path() -> PathBuf {
    RUST_LIBRARY
        .get_or_init(|| {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let output_dir = root.join("target/differential");
            let library = output_dir.join("libdriver.so");
            fs::create_dir_all(&output_dir).expect("create Rust test-library directory");
            let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
            let status = Command::new(rustc)
                .args([
                    "--edition=2021",
                    "--crate-type=cdylib",
                    "--crate-name=driver",
                ])
                .arg(root.join("src/lib.rs"))
                .arg("-o")
                .arg(&library)
                .status()
                .expect("invoke rustc for test cdylib");
            assert!(status.success(), "failed to build Rust test cdylib");
            library
        })
        .clone()
}

fn temporary_path(kind: &str) -> PathBuf {
    let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver-differential-{}-{sequence}.{kind}",
        std::process::id()
    ))
}

fn input_file(input: &Input, path: &Path) -> File {
    match input {
        Input::Bytes(bytes) => {
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .read(true)
                .write(true)
                .open(path)
                .expect("open test input");
            file.write_all(bytes).expect("write test input");
            file.seek(SeekFrom::Start(0)).expect("rewind test input");
            file
        }
        Input::ReadError => OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
            .expect("open write-only test input"),
    }
}

fn capture<R>(input: &Input, call: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let _lock = STDIO_LOCK.lock().expect("lock process stdio");
    let input_path = temporary_path("in");
    let output_path = temporary_path("out");
    let input_file = input_file(input, &input_path);
    let mut output_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&output_path)
        .expect("open test output");

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        __fpurge(stdin);
        clearerr(stdin);

        let saved_stdin = dup(0);
        let saved_stdout = dup(1);
        assert!(saved_stdin >= 0 && saved_stdout >= 0);
        assert_eq!(dup2(input_file.as_raw_fd(), 0), 0);
        assert_eq!(dup2(output_file.as_raw_fd(), 1), 1);
        clearerr(stdin);

        let result = call();

        assert_eq!(fflush(ptr::null_mut()), 0);
        __fpurge(stdin);
        clearerr(stdin);
        assert_eq!(dup2(saved_stdin, 0), 0);
        assert_eq!(dup2(saved_stdout, 1), 1);
        assert_eq!(close(saved_stdin), 0);
        assert_eq!(close(saved_stdout), 0);

        output_file
            .seek(SeekFrom::Start(0))
            .expect("rewind test output");
        let mut output = Vec::new();
        output_file
            .read_to_end(&mut output)
            .expect("read test output");

        drop(input_file);
        drop(output_file);
        fs::remove_file(input_path).expect("remove test input");
        fs::remove_file(output_path).expect("remove test output");
        (result, output)
    }
}

unsafe fn run_void(path: &Path, symbol: &[u8], inputs: &[Input]) -> Vec<Vec<u8>> {
    let library = Library::new(path).expect("load shared library");
    let function = *library.get::<VoidFn>(symbol).expect("load void symbol");
    inputs
        .iter()
        .map(|input| capture(input, || function()).1)
        .collect()
}

unsafe fn run_main(path: &Path, cases: &[(Input, c_int)]) -> Vec<(c_int, Vec<u8>)> {
    let library = Library::new(path).expect("load shared library");
    let function = *library.get::<MainFn>(b"main").expect("load main symbol");
    cases
        .iter()
        .map(|(input, argc)| capture(input, || function(*argc, ptr::null_mut())))
        .collect()
}

fn compare_void(symbol: &[u8], inputs: &[Input]) -> Vec<Vec<u8>> {
    let c = unsafe { run_void(&c_library_path(), symbol, inputs) };
    let rust = unsafe { run_void(&rust_library_path(), symbol, inputs) };
    assert_eq!(
        rust,
        c,
        "{} output diverged",
        String::from_utf8_lossy(symbol)
    );
    c
}

fn compare_main(cases: &[(Input, c_int)]) -> Vec<(c_int, Vec<u8>)> {
    let c = unsafe { run_main(&c_library_path(), cases) };
    let rust = unsafe { run_main(&rust_library_path(), cases) };
    assert_eq!(rust, c, "main result or output diverged");
    c
}

fn random_indices(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed);
    (0..64).map(|_| rng.index()).collect()
}

fn good_prefix() -> Vec<u8> {
    b"0\n0\n0\n0\n0\n0\n0\n1\n0\n0\n".to_vec()
}

#[test]
fn config_01_print_line_non_null() {
    let mut rng = Rng::new(0x0101_5eed);
    let mut values = vec![CString::new("").unwrap()];
    for _ in 0..128 {
        let len = (rng.next_u32() % 65) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| b' ' + (rng.next_u32() % 95) as u8)
            .collect();
        values.push(CString::new(bytes).unwrap());
    }

    unsafe fn run(path: &Path, values: &[CString]) -> Vec<Vec<u8>> {
        let library = Library::new(path).expect("load shared library");
        let function = *library
            .get::<PrintLineFn>(b"printLine")
            .expect("load printLine");
        values
            .iter()
            .map(|value| capture(&Input::Bytes(Vec::new()), || function(value.as_ptr())).1)
            .collect()
    }

    let c = unsafe { run(&c_library_path(), &values) };
    let rust = unsafe { run(&rust_library_path(), &values) };
    assert_eq!(rust, c);
}

#[test]
fn config_02_print_int_line_all_int_shapes() {
    let mut rng = Rng::new(0x0202_5eed);
    let mut values = vec![c_int::MIN, -1, 0, 1, c_int::MAX];
    values.extend((0..128).map(|_| rng.next_u32() as c_int));

    unsafe fn run(path: &Path, values: &[c_int]) -> Vec<Vec<u8>> {
        let library = Library::new(path).expect("load shared library");
        let function = *library
            .get::<PrintIntLineFn>(b"printIntLine")
            .expect("load printIntLine");
        values
            .iter()
            .map(|value| capture(&Input::Bytes(Vec::new()), || function(*value)).1)
            .collect()
    }

    let c = unsafe { run(&c_library_path(), &values) };
    let rust = unsafe { run(&rust_library_path(), &values) };
    assert_eq!(rust, c);
}

#[test]
fn config_03_bad_newline_terminated_valid_index() {
    let inputs: Vec<_> = random_indices(0x0303_5eed)
        .into_iter()
        .map(|index| Input::Bytes(format!(" +{index}\n").into_bytes()))
        .collect();
    compare_void(b"bad", &inputs);
}

#[test]
fn config_04_bad_eof_terminated_valid_index() {
    let inputs: Vec<_> = random_indices(0x0404_5eed)
        .into_iter()
        .map(|index| Input::Bytes(format!("\t{index}").into_bytes()))
        .collect();
    compare_void(b"bad", &inputs);
}

#[test]
fn config_05_bad_exactly_thirteen_bytes() {
    let inputs: Vec<_> = random_indices(0x0505_5eed)
        .into_iter()
        .map(|index| Input::Bytes(format!("{:012}{index}", 0).into_bytes()))
        .collect();
    assert!(inputs.iter().all(|input| match input {
        Input::Bytes(bytes) => bytes.len() == 13,
        Input::ReadError => false,
    }));
    compare_void(b"bad", &inputs);
}

#[test]
fn config_06_bad_one_past_array_bound() {
    let outputs = compare_void(b"bad", &[Input::Bytes(b"10\n".to_vec())]);
    assert_eq!(outputs[0], b"0\n0\n0\n0\n0\n0\n0\n0\n0\n0\n");
}

#[test]
fn config_07_good_newline_terminated_valid_index() {
    let inputs: Vec<_> = random_indices(0x0707_5eed)
        .into_iter()
        .map(|index| Input::Bytes(format!(" {index}\n").into_bytes()))
        .collect();
    compare_void(b"good", &inputs);
}

#[test]
fn config_08_good_eof_terminated_valid_index() {
    let inputs: Vec<_> = random_indices(0x0808_5eed)
        .into_iter()
        .map(|index| Input::Bytes(index.to_string().into_bytes()))
        .collect();
    compare_void(b"good", &inputs);
}

#[test]
fn config_09_good_exactly_thirteen_bytes() {
    let inputs: Vec<_> = random_indices(0x0909_5eed)
        .into_iter()
        .map(|index| Input::Bytes(format!("{:012}{index}", 0).into_bytes()))
        .collect();
    compare_void(b"good", &inputs);
}

#[test]
fn config_10_main_two_valid_lines_and_ignored_arguments() {
    let first = random_indices(0x1010_5eed);
    let second = random_indices(0x1010_beef);
    let argc_values = [c_int::MIN, -1, 0, 1, c_int::MAX];
    let cases: Vec<_> = first
        .into_iter()
        .zip(second)
        .enumerate()
        .map(|(position, (good_index, bad_index))| {
            (
                Input::Bytes(format!("{good_index}\n{bad_index}\n").into_bytes()),
                argc_values[position % argc_values.len()],
            )
        })
        .collect();
    let results = compare_main(&cases);
    assert!(results.iter().all(|(result, _)| *result == 0));
}

#[test]
fn config_11_main_one_valid_line_then_eof() {
    let cases: Vec<_> = random_indices(0x1111_5eed)
        .into_iter()
        .map(|index| (Input::Bytes(format!("{index}\n").into_bytes()), 0))
        .collect();
    compare_main(&cases);
}

#[test]
fn config_12_main_empty_input() {
    let results = compare_main(&[(Input::Bytes(Vec::new()), 0)]);
    assert_eq!(results[0].0, 0);
}

#[test]
fn config_13_main_thirteen_byte_first_read_then_remainder() {
    let first = random_indices(0x1313_5eed);
    let second = random_indices(0x1313_beef);
    let cases: Vec<_> = first
        .into_iter()
        .zip(second)
        .map(|(good_index, bad_index)| {
            (
                Input::Bytes(format!("{:012}{good_index}{bad_index}\n", 0).into_bytes()),
                1,
            )
        })
        .collect();
    compare_main(&cases);
}

#[test]
fn error_01_print_line_null() {
    unsafe fn run(path: &Path) -> Vec<u8> {
        let library = Library::new(path).expect("load shared library");
        let function = *library
            .get::<PrintLineFn>(b"printLine")
            .expect("load printLine");
        capture(&Input::Bytes(Vec::new()), || function(ptr::null())).1
    }

    let c = unsafe { run(&c_library_path()) };
    let rust = unsafe { run(&rust_library_path()) };
    assert_eq!(c, b"");
    assert_eq!(rust, c);
}

#[test]
fn error_02_bad_fgets_null() {
    let expected = b"fgets() failed.\nERROR: Array index is negative.\n";
    let outputs = compare_void(b"bad", &[Input::Bytes(Vec::new()), Input::ReadError]);
    assert!(outputs.iter().all(|output| output == expected));
}

#[test]
fn error_03_bad_negative_index() {
    let mut rng = Rng::new(0x0303_dead);
    let inputs: Vec<_> = (0..64)
        .map(|_| {
            let magnitude = 1 + rng.next_u32() % 999_999;
            Input::Bytes(format!("-{magnitude}\n").into_bytes())
        })
        .collect();
    let outputs = compare_void(b"bad", &inputs);
    assert!(outputs
        .iter()
        .all(|output| output == b"ERROR: Array index is negative.\n"));
}

#[test]
fn error_04_good_fgets_null() {
    let mut expected = good_prefix();
    expected.extend_from_slice(b"fgets() failed.\nERROR: Array index is out-of-bounds\n");
    let outputs = compare_void(b"good", &[Input::Bytes(Vec::new()), Input::ReadError]);
    assert!(outputs.iter().all(|output| output == &expected));
}

#[test]
fn error_05_good_negative_index() {
    let mut rng = Rng::new(0x0505_dead);
    let inputs: Vec<_> = (0..64)
        .map(|_| {
            let magnitude = 1 + rng.next_u32() % 999_999;
            Input::Bytes(format!("-{magnitude}\n").into_bytes())
        })
        .collect();
    let mut expected = good_prefix();
    expected.extend_from_slice(b"ERROR: Array index is out-of-bounds\n");
    let outputs = compare_void(b"good", &inputs);
    assert!(outputs.iter().all(|output| output == &expected));
}

#[test]
fn error_06_good_index_at_or_above_ten() {
    let mut rng = Rng::new(0x0606_dead);
    let inputs: Vec<_> = std::iter::once(10)
        .chain((0..63).map(|_| 10 + rng.next_u32() % 999_989))
        .map(|index| Input::Bytes(format!("{index}\n").into_bytes()))
        .collect();
    let mut expected = good_prefix();
    expected.extend_from_slice(b"ERROR: Array index is out-of-bounds\n");
    let outputs = compare_void(b"good", &inputs);
    assert!(outputs.iter().all(|output| output == &expected));
}
