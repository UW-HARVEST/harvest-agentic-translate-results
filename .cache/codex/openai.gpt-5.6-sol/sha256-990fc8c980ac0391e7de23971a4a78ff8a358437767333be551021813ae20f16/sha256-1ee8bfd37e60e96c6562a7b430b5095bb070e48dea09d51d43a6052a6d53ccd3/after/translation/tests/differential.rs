use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::ptr;
use std::sync::Mutex;

type Foo = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
type Driver = unsafe extern "C" fn(*const c_char);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const CASES_PER_CONFIGURATION: usize = 128;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

struct Api {
    _library: Library,
    foo: Foo,
    driver: Driver,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let foo = unsafe { *library.get::<Foo>(b"foo\0").expect("missing foo export") };
        let driver = unsafe {
            *library
                .get::<Driver>(b"driver\0")
                .expect("missing driver export")
        };
        Self {
            _library: library,
            foo,
            driver,
        }
    }
}

#[derive(Clone, Copy)]
enum Count {
    Zero,
    One,
    Many,
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn usize(&mut self, upper_exclusive: usize) -> usize {
        (self.next_u64() as usize) % upper_exclusive
    }

    fn byte_excluding(&mut self, excluded: &[u8]) -> u8 {
        loop {
            let byte = (self.next_u64() >> 32) as u8;
            if byte != 0 && !excluded.contains(&byte) {
                return byte;
            }
        }
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            values.swap(index, self.usize(index + 1));
        }
    }
}

fn c_so() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .expect("C shared library must be built before running tests")
}

fn rust_so() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/libdriver.so")
        .canonicalize()
        .expect("run `cargo build --release` before the differential tests")
}

fn load_apis() -> (Api, Api) {
    unsafe { (Api::load(&c_so()), Api::load(&rust_so())) }
}

fn foo_case(rng: &mut Rng, count: Count, empty: bool) -> (CString, c_char, c_int) {
    let target = rng.byte_excluding(&[]);
    if empty {
        return (CString::new(Vec::new()).unwrap(), target as c_char, 0);
    }

    let occurrences = match count {
        Count::Zero => 0,
        Count::One => 1,
        Count::Many => 2 + rng.usize(7),
    };
    let length = occurrences.max(1) + rng.usize(96);
    let mut bytes = (0..length)
        .map(|_| rng.byte_excluding(&[target]))
        .collect::<Vec<_>>();
    let mut positions = (0..length).collect::<Vec<_>>();
    rng.shuffle(&mut positions);
    for position in positions.into_iter().take(occurrences) {
        bytes[position] = target;
    }

    (
        CString::new(bytes).unwrap(),
        target as c_char,
        occurrences as c_int,
    )
}

fn count_value(rng: &mut Rng, count: Count) -> usize {
    match count {
        Count::Zero => 0,
        Count::One => 1,
        Count::Many => 2 + rng.usize(7),
    }
}

fn driver_case(rng: &mut Rng, a_shape: Count, x_shape: Count) -> (CString, usize, usize) {
    let a_count = count_value(rng, a_shape);
    let x_count = count_value(rng, x_shape);
    let minimum_filler = usize::from(a_count + x_count == 0);
    let filler_count = minimum_filler + rng.usize(96);
    let mut bytes = Vec::with_capacity(a_count + x_count + filler_count);
    bytes.extend(std::iter::repeat_n(b'A', a_count));
    bytes.extend(std::iter::repeat_n(b'x', x_count));
    bytes.extend((0..filler_count).map(|_| rng.byte_excluding(&[b'A', b'x'])));
    rng.shuffle(&mut bytes);
    (CString::new(bytes).unwrap(), a_count, x_count)
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut pipe_fds = [-1; 2];
    let pipe_result = unsafe { pipe(pipe_fds.as_mut_ptr()) };
    assert_eq!(pipe_result, 0, "pipe failed");
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0, "dup failed");

    unsafe {
        fflush(ptr::null_mut());
        assert_eq!(dup2(pipe_fds[1], STDOUT_FILENO), STDOUT_FILENO);
        close(pipe_fds[1]);
    }

    call();

    unsafe {
        fflush(ptr::null_mut());
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        close(saved_stdout);
    }

    let mut output = Vec::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_end(&mut output)
        .expect("read captured stdout");
    output
}

#[test]
fn all_c_symbols_load_from_both_shared_libraries() {
    let _apis = load_apis();
}

#[test]
fn foo_matches_for_every_configuration_row() {
    let (c_api, rust_api) = load_apis();
    let configurations = [
        ("row 1: empty, zero", Count::Zero, true),
        ("row 2: nonempty, zero", Count::Zero, false),
        ("row 3: exactly one", Count::One, false),
        ("row 4: multiple", Count::Many, false),
    ];
    let mut rng = Rng::new(0x8065_05aa_e298_7713);

    for (label, count, empty) in configurations {
        for case_index in 0..CASES_PER_CONFIGURATION {
            let (input, target, expected) = foo_case(&mut rng, count, empty);
            let c_result = unsafe { (c_api.foo)(input.as_ptr(), target) };
            let rust_result = unsafe { (rust_api.foo)(input.as_ptr(), target) };
            assert_eq!(c_result, expected, "{label}, case {case_index}: C");
            assert_eq!(
                rust_result, c_result,
                "{label}, case {case_index}: Rust differs from C"
            );
        }
    }
}

fn driver_corpus(api: &Api) -> Vec<u8> {
    let configurations = [
        ("row 6", Count::Zero, Count::Zero),
        ("row 7", Count::Zero, Count::One),
        ("row 8", Count::Zero, Count::Many),
        ("row 9", Count::One, Count::Zero),
        ("row 10", Count::One, Count::One),
        ("row 11", Count::One, Count::Many),
        ("row 12", Count::Many, Count::Zero),
        ("row 13", Count::Many, Count::One),
        ("row 14", Count::Many, Count::Many),
    ];
    let mut rng = Rng::new(0xc567_58a1_4e2b_09f3);
    let mut corpus = Vec::new();

    let empty = CString::new(Vec::new()).unwrap();
    let empty_output = capture_stdout(|| unsafe { (api.driver)(empty.as_ptr()) });
    assert_eq!(empty_output, b"A: 0\nx: 0\n", "row 5");
    corpus.extend_from_slice(&(empty_output.len() as u32).to_le_bytes());
    corpus.extend_from_slice(&empty_output);

    for (label, a_shape, x_shape) in configurations {
        for case_index in 0..CASES_PER_CONFIGURATION {
            let (input, a_count, x_count) = driver_case(&mut rng, a_shape, x_shape);
            let output = capture_stdout(|| unsafe { (api.driver)(input.as_ptr()) });
            let expected = format!("A: {a_count}\nx: {x_count}\n").into_bytes();
            assert_eq!(output, expected, "{label}, case {case_index}");
            corpus.extend_from_slice(&(output.len() as u32).to_le_bytes());
            corpus.extend_from_slice(&output);
        }
    }
    corpus
}

fn run_driver_corpus(library: &Path, label: &str) -> Vec<u8> {
    let output_path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{label}.bin",
        std::process::id()
    ));
    let child = Command::new(std::env::current_exe().expect("test executable path"))
        .args(["--exact", "driver_corpus_child", "--ignored", "--nocapture"])
        .env("DIFF_DRIVER_LIBRARY", library)
        .env("DIFF_DRIVER_OUTPUT", &output_path)
        .output()
        .expect("run driver corpus child");
    assert!(
        child.status.success(),
        "{label} driver corpus failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    let corpus = std::fs::read(&output_path).expect("read driver corpus");
    std::fs::remove_file(output_path).expect("remove driver corpus");
    corpus
}

#[test]
fn driver_matches_for_every_configuration_row() {
    let c_corpus = run_driver_corpus(&c_so(), "c");
    let rust_corpus = run_driver_corpus(&rust_so(), "rust");
    assert_eq!(rust_corpus, c_corpus, "Rust driver corpus differs from C");
}

fn null_call_outcome(library: &Path, symbol: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().expect("test executable path"))
        .args(["--exact", "null_pointer_child", "--ignored", "--nocapture"])
        .env("DIFF_NULL_LIBRARY", library)
        .env("DIFF_NULL_SYMBOL", symbol)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run null-pointer child")
}

#[test]
fn null_pointers_have_matching_process_outcomes() {
    use std::os::unix::process::ExitStatusExt;

    for symbol in ["foo", "driver"] {
        let c_status = null_call_outcome(&c_so(), symbol);
        let rust_status = null_call_outcome(&rust_so(), symbol);
        assert!(
            !c_status.success(),
            "C {symbol}(NULL) unexpectedly succeeded"
        );
        assert_eq!(
            (rust_status.code(), rust_status.signal()),
            (c_status.code(), c_status.signal()),
            "{symbol}(NULL) process outcome differs"
        );
    }
}

#[test]
#[ignore = "isolated stdout-capture target for driver differential test"]
fn driver_corpus_child() {
    let Ok(library_path) = std::env::var("DIFF_DRIVER_LIBRARY") else {
        return;
    };
    let output_path = std::env::var("DIFF_DRIVER_OUTPUT").expect("driver output path");
    let api = unsafe { Api::load(Path::new(&library_path)) };
    std::fs::write(output_path, driver_corpus(&api)).expect("write driver corpus");
}

#[test]
#[ignore = "subprocess crash target for null-pointer differential test"]
fn null_pointer_child() {
    let Ok(library_path) = std::env::var("DIFF_NULL_LIBRARY") else {
        return;
    };
    let symbol = std::env::var("DIFF_NULL_SYMBOL").expect("null child symbol");
    let api = unsafe { Api::load(Path::new(&library_path)) };
    match symbol.as_str() {
        "foo" => unsafe {
            (api.foo)(ptr::null(), b'A' as c_char);
        },
        "driver" => unsafe {
            (api.driver)(ptr::null());
        },
        _ => panic!("unknown null child symbol: {symbol}"),
    }
}
