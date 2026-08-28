use libloading::Library;
use std::env;
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::ptr;

type SliceFn = unsafe extern "C" fn(*mut c_char, *mut c_int, *mut c_int) -> c_int;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const RANDOM_CASES_PER_ROW: usize = 64;

#[derive(Debug, Eq, PartialEq)]
struct Observation {
    result: c_int,
    stdout: Vec<u8>,
}

#[derive(Clone, Debug)]
struct Case {
    bytes: Vec<u8>,
    start: Option<c_int>,
    stop: Option<c_int>,
}

struct Api {
    _library: Library,
    slice: SliceFn,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let slice = unsafe {
            *library.get::<SliceFn>(b"slice\0").unwrap_or_else(|error| {
                panic!("failed to load slice from {}: {error}", path.display())
            })
        };
        Self {
            _library: library,
            slice,
        }
    }

    unsafe fn invoke(&self, case: &Case) -> Observation {
        let mut bytes = case.bytes.clone();
        bytes.push(0);
        let mut start = case.start;
        let mut stop = case.stop;
        let start_ptr = start
            .as_mut()
            .map_or(ptr::null_mut(), |value| value as *mut c_int);
        let stop_ptr = stop
            .as_mut()
            .map_or(ptr::null_mut(), |value| value as *mut c_int);

        let capture = unsafe { StdoutCapture::start() };
        let result = unsafe { (self.slice)(bytes.as_mut_ptr().cast(), start_ptr, stop_ptr) };
        let stdout = unsafe { capture.finish() };
        Observation { result, stdout }
    }
}

struct StdoutCapture {
    saved_stdout: c_int,
    read_fd: c_int,
}

impl StdoutCapture {
    unsafe fn start() -> Self {
        assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);

        let mut fds = [-1; 2];
        assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);
        let saved_stdout = unsafe { dup(STDOUT_FILENO) };
        assert!(saved_stdout >= 0);
        assert_eq!(unsafe { dup2(fds[1], STDOUT_FILENO) }, STDOUT_FILENO);
        assert_eq!(unsafe { close(fds[1]) }, 0);

        Self {
            saved_stdout,
            read_fd: fds[0],
        }
    }

    unsafe fn finish(self) -> Vec<u8> {
        assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0);
        assert_eq!(
            unsafe { dup2(self.saved_stdout, STDOUT_FILENO) },
            STDOUT_FILENO
        );
        assert_eq!(unsafe { close(self.saved_stdout) }, 0);

        let mut output = Vec::new();
        let mut reader = unsafe { File::from_raw_fd(self.read_fd) };
        reader.read_to_end(&mut output).unwrap();
        output
    }
}

#[derive(Clone)]
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

    fn usize_in(&mut self, low: usize, high_inclusive: usize) -> usize {
        assert!(low <= high_inclusive);
        low + (self.next_u64() as usize % (high_inclusive - low + 1))
    }

    fn nonzero_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| (self.usize_in(1, u8::MAX as usize)) as u8)
            .collect()
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libString_Slice.so")
}

fn rust_library_path() -> PathBuf {
    env::var_os("RUST_SLICE_DYLIB")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libString_Slice.so")
        })
}

fn multi_bytes(rng: &mut Rng, minimum: usize) -> Vec<u8> {
    let len = rng.usize_in(minimum, 64);
    rng.nonzero_bytes(len)
}

fn embedded_nul_bytes(rng: &mut Rng) -> (Vec<u8>, usize) {
    let effective_len = rng.usize_in(1, 32);
    let trailing_len = rng.usize_in(1, 32);
    let mut bytes = rng.nonzero_bytes(effective_len);
    bytes.push(0);
    bytes.extend(rng.nonzero_bytes(trailing_len));
    (bytes, effective_len)
}

fn valid_case(row: usize, rng: &mut Rng) -> Case {
    match row {
        1 => Case {
            bytes: vec![],
            start: None,
            stop: None,
        },
        2 => Case {
            bytes: vec![],
            start: Some(0),
            stop: None,
        },
        3 => Case {
            bytes: rng.nonzero_bytes(1),
            start: None,
            stop: None,
        },
        4 => Case {
            bytes: rng.nonzero_bytes(1),
            start: Some(0),
            stop: None,
        },
        5 => Case {
            bytes: rng.nonzero_bytes(1),
            start: Some(1),
            stop: None,
        },
        6 => Case {
            bytes: rng.nonzero_bytes(1),
            start: None,
            stop: Some(1),
        },
        7 => Case {
            bytes: rng.nonzero_bytes(1),
            start: Some(0),
            stop: Some(1),
        },
        8 => Case {
            bytes: multi_bytes(rng, 2),
            start: None,
            stop: None,
        },
        9 => Case {
            bytes: multi_bytes(rng, 2),
            start: Some(0),
            stop: None,
        },
        10 => {
            let bytes = multi_bytes(rng, 2);
            let start = rng.usize_in(1, bytes.len() - 1) as c_int;
            Case {
                bytes,
                start: Some(start),
                stop: None,
            }
        }
        11 => {
            let bytes = multi_bytes(rng, 2);
            let start = bytes.len() as c_int;
            Case {
                bytes,
                start: Some(start),
                stop: None,
            }
        }
        12 => {
            let bytes = multi_bytes(rng, 2);
            let stop = rng.usize_in(1, bytes.len() - 1) as c_int;
            Case {
                bytes,
                start: None,
                stop: Some(stop),
            }
        }
        13 => {
            let bytes = multi_bytes(rng, 2);
            let stop = bytes.len() as c_int;
            Case {
                bytes,
                start: None,
                stop: Some(stop),
            }
        }
        14 => {
            let bytes = multi_bytes(rng, 2);
            let stop = bytes.len() as c_int;
            Case {
                bytes,
                start: Some(0),
                stop: Some(stop),
            }
        }
        15 => {
            let bytes = multi_bytes(rng, 3);
            let start = rng.usize_in(1, bytes.len() - 2);
            let stop = rng.usize_in(start + 1, bytes.len() - 1);
            Case {
                bytes,
                start: Some(start as c_int),
                stop: Some(stop as c_int),
            }
        }
        16 => {
            let (bytes, _) = embedded_nul_bytes(rng);
            Case {
                bytes,
                start: None,
                stop: None,
            }
        }
        17 => {
            let (bytes, effective_len) = embedded_nul_bytes(rng);
            let start = rng.usize_in(0, effective_len) as c_int;
            Case {
                bytes,
                start: Some(start),
                stop: None,
            }
        }
        18 => {
            let (bytes, effective_len) = embedded_nul_bytes(rng);
            let stop = rng.usize_in(1, effective_len) as c_int;
            Case {
                bytes,
                start: None,
                stop: Some(stop),
            }
        }
        19 => {
            let (bytes, effective_len) = embedded_nul_bytes(rng);
            let start = rng.usize_in(0, effective_len - 1);
            let stop = rng.usize_in(start + 1, effective_len);
            Case {
                bytes,
                start: Some(start as c_int),
                stop: Some(stop as c_int),
            }
        }
        _ => panic!("unknown CONFIGS.md row {row}"),
    }
}

fn invalid_case(row: usize, iteration: usize, rng: &mut Rng) -> Case {
    let len = rng.usize_in(0, 32);
    let bytes = rng.nonzero_bytes(len);
    let len = bytes.len() as c_int;

    match row {
        1 => {
            let start = match iteration % 5 {
                0 => -1,
                1 => c_int::MIN,
                2 => len + 1,
                3 => len + 2 + rng.usize_in(0, 1000) as c_int,
                _ => c_int::MAX,
            };
            Case {
                bytes,
                start: Some(start),
                stop: None,
            }
        }
        2 => {
            let stop = match iteration % 5 {
                0 => -1,
                1 => c_int::MIN,
                2 => len + 1,
                3 => len + 2 + rng.usize_in(0, 1000) as c_int,
                _ => c_int::MAX,
            };
            Case {
                bytes,
                start: (iteration % 2 == 0).then_some(0),
                stop: Some(stop),
            }
        }
        3 => {
            let start = rng.usize_in(0, bytes.len()) as c_int;
            let stop = rng.usize_in(0, start as usize) as c_int;
            Case {
                bytes,
                start: (iteration % 3 != 0 || start != 0).then_some(start),
                stop: Some(stop),
            }
        }
        _ => panic!("unknown ERRORS.md row {row}"),
    }
}

fn compare_case(c_api: &Api, rust_api: &Api, case: &Case, label: &str) -> Observation {
    let c_observation = unsafe { c_api.invoke(case) };
    let rust_observation = unsafe { rust_api.invoke(case) };
    assert_eq!(
        rust_observation, c_observation,
        "{label} diverged for {case:?}"
    );
    c_observation
}

#[test]
fn valid_configuration_rows_match() {
    let c_api = unsafe { Api::load(&c_library_path()) };
    let rust_api = unsafe { Api::load(&rust_library_path()) };
    let mut rng = Rng::new(0x6f12_44ca_d851_9e37);

    for row in 1..=19 {
        for iteration in 0..RANDOM_CASES_PER_ROW {
            let case = valid_case(row, &mut rng);
            let observation = compare_case(
                &c_api,
                &rust_api,
                &case,
                &format!("CONFIGS.md row {row}, iteration {iteration}"),
            );
            assert_eq!(observation.result, 0);
        }
    }
}

#[test]
fn explicit_error_rows_match() {
    let c_api = unsafe { Api::load(&c_library_path()) };
    let rust_api = unsafe { Api::load(&rust_library_path()) };
    let mut rng = Rng::new(0x16be_7a91_052c_f403);
    let expected_stdout: [&[u8]; 3] = [
        b"Error: start is off the end of the string!\n",
        b"Error: stop is off the end of the string!\n",
        b"Error: stop must come after start!\n",
    ];

    for row in 1..=3 {
        for iteration in 0..RANDOM_CASES_PER_ROW {
            let case = invalid_case(row, iteration, &mut rng);
            let observation = compare_case(
                &c_api,
                &rust_api,
                &case,
                &format!("ERRORS.md row {row}, iteration {iteration}"),
            );
            assert_eq!(observation.result, 1);
            assert_eq!(observation.stdout, expected_stdout[row - 1]);
        }
    }
}

fn run_null_child(path: &Path, start_present: bool, stop_present: bool) -> ExitStatus {
    Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("null_mystr_child")
        .arg("--nocapture")
        .env("SLICE_NULL_DYLIB", path)
        .env("SLICE_NULL_START", if start_present { "1" } else { "0" })
        .env("SLICE_NULL_STOP", if stop_present { "1" } else { "0" })
        .status()
        .unwrap()
}

#[test]
fn null_mystr_termination_matches() {
    for start_present in [false, true] {
        for stop_present in [false, true] {
            let c_status = run_null_child(&c_library_path(), start_present, stop_present);
            let rust_status = run_null_child(&rust_library_path(), start_present, stop_present);
            assert_eq!(c_status.signal(), Some(11));
            assert_eq!(
                rust_status.signal(),
                c_status.signal(),
                "ERRORS.md row 4 differs with start_present={start_present}, \
                 stop_present={stop_present}"
            );
        }
    }
}

#[test]
fn null_mystr_child() {
    let Some(path) = env::var_os("SLICE_NULL_DYLIB") else {
        return;
    };
    let api = unsafe { Api::load(Path::new(&path)) };
    let mut start = 0;
    let mut stop = 0;
    let start_ptr = if env::var("SLICE_NULL_START").as_deref() == Ok("1") {
        &mut start
    } else {
        ptr::null_mut()
    };
    let stop_ptr = if env::var("SLICE_NULL_STOP").as_deref() == Ok("1") {
        &mut stop
    } else {
        ptr::null_mut()
    };

    unsafe {
        (api.slice)(ptr::null_mut(), start_ptr, stop_ptr);
    }
    panic!("slice unexpectedly returned for a null string");
}
