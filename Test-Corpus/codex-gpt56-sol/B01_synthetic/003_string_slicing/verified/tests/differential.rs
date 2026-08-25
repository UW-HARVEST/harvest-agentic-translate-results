use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::fs::{self, File};
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

type DriverMain = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
const CASES_PER_ROW: usize = 64;

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

#[derive(Debug, Eq, PartialEq)]
struct Outcome {
    status: c_int,
    stdout: Vec<u8>,
}

struct Arguments {
    _storage: Vec<Vec<u8>>,
    pointers: Vec<*mut c_char>,
}

impl Arguments {
    fn new(values: &[Vec<u8>]) -> Self {
        let mut storage: Vec<Vec<u8>> = values
            .iter()
            .map(|value| {
                assert!(
                    !value.contains(&0),
                    "arguments cannot contain interior NULs"
                );
                let mut terminated = value.clone();
                terminated.push(0);
                terminated
            })
            .collect();
        let mut pointers: Vec<*mut c_char> = storage
            .iter_mut()
            .map(|value| value.as_mut_ptr().cast())
            .collect();
        pointers.push(ptr::null_mut());
        Self {
            _storage: storage,
            pointers,
        }
    }

    fn argc(&self) -> c_int {
        (self.pointers.len() - 1) as c_int
    }
}

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

    fn range(&mut self, start: usize, end: usize) -> usize {
        assert!(start < end);
        start + self.next() as usize % (end - start)
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| (self.range(0, u8::MAX as usize) + 1) as u8)
            .collect()
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn build_c_library() -> PathBuf {
    let output = manifest_dir().join("c_src/build/libdriver_c.so");
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    let result = Command::new("cc")
        .args(["-shared", "-fPIC", "-o"])
        .arg(&output)
        .arg(manifest_dir().join("c_src/src/main.c"))
        .output()
        .expect("failed to run cc");
    assert!(
        result.status.success(),
        "C shared-library build failed:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
    output
}

fn rust_library() -> PathBuf {
    let executable = std::env::current_exe().unwrap();
    let deps = executable.parent().unwrap();
    let direct_candidates = [
        deps.join("libdriver.so"),
        deps.parent().unwrap().join("libdriver.so"),
    ];
    if let Some(path) = direct_candidates.into_iter().find(|path| path.is_file()) {
        return path;
    }

    let mut candidates: Vec<PathBuf> = fs::read_dir(deps)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("libdriver") && name.ends_with(".so"))
        })
        .collect();
    candidates.sort_by_key(|path| fs::metadata(path).and_then(|meta| meta.modified()).ok());
    candidates
        .pop()
        .unwrap_or_else(|| panic!("Rust cdylib was not found beside {}", executable.display()))
}

unsafe fn invoke(library: &Library, arguments: &mut Arguments, argc: Option<c_int>) -> Outcome {
    let function: Symbol<DriverMain> = library.get(b"main").unwrap();
    let _guard = STDOUT_LOCK.lock().unwrap();
    assert_eq!(fflush(ptr::null_mut()), 0);

    let mut pipe_fds = [-1, -1];
    assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
    let saved_stdout = dup(1);
    assert!(saved_stdout >= 0);
    assert_eq!(dup2(pipe_fds[1], 1), 1);
    assert_eq!(close(pipe_fds[1]), 0);

    let status = function(
        argc.unwrap_or_else(|| arguments.argc()),
        arguments.pointers.as_mut_ptr(),
    );

    assert_eq!(fflush(ptr::null_mut()), 0);
    assert_eq!(dup2(saved_stdout, 1), 1);
    assert_eq!(close(saved_stdout), 0);

    let mut stdout = Vec::new();
    File::from_raw_fd(pipe_fds[0])
        .read_to_end(&mut stdout)
        .unwrap();
    Outcome { status, stdout }
}

fn compare(id: &str, c_library: &Library, rust_library: &Library, values: &[Vec<u8>]) -> Outcome {
    let mut c_arguments = Arguments::new(values);
    let mut rust_arguments = Arguments::new(values);
    let c = unsafe { invoke(c_library, &mut c_arguments, None) };
    let rust = unsafe { invoke(rust_library, &mut rust_arguments, None) };
    assert_eq!(rust, c, "{id}: values={values:?}");
    c
}

fn expect_error(
    id: &str,
    c_library: &Library,
    rust_library: &Library,
    values: &[Vec<u8>],
    expected_stdout: &[u8],
) {
    let outcome = compare(id, c_library, rust_library, values);
    assert_eq!(outcome.status, 1, "{id}");
    assert_eq!(outcome.stdout, expected_stdout, "{id}");
}

fn decimal(value: usize) -> Vec<u8> {
    value.to_string().into_bytes()
}

fn values(string: Vec<u8>, extra: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut result = vec![b"driver".to_vec(), string];
    result.extend_from_slice(extra);
    result
}

fn compare_alias_error(id: &str, c_library: &Library, rust_library: &Library) {
    fn aliased_arguments() -> Arguments {
        let mut arguments = Arguments::new(&[
            b"driver".to_vec(),
            b"abcdef".to_vec(),
            b"1x".to_vec(),
            b"unused".to_vec(),
        ]);
        arguments.pointers[3] = unsafe { arguments.pointers[2].add(1) };
        arguments
    }

    let mut c_arguments = aliased_arguments();
    let mut rust_arguments = aliased_arguments();
    let c = unsafe { invoke(c_library, &mut c_arguments, None) };
    let rust = unsafe { invoke(rust_library, &mut rust_arguments, None) };
    assert_eq!(rust, c, "{id}");
    assert_eq!(
        c,
        Outcome {
            status: 1,
            stdout: b"Third argument must be an integer!".to_vec(),
        },
        "{id}"
    );
}

#[test]
fn differential_surface() {
    let c_path = build_c_library();
    let rust_path = rust_library();
    let c_library = unsafe { Library::new(&c_path).unwrap() };
    let rust_library = unsafe { Library::new(&rust_path).unwrap() };
    let mut rng = Rng::new(0x5eed_d1ff_e2e0_2025);

    // C01: argc 2, empty.
    for _ in 0..CASES_PER_ROW {
        compare("C01", &c_library, &rust_library, &values(vec![], &[]));
    }

    // C02: argc 2, one byte.
    for _ in 0..CASES_PER_ROW {
        let input = rng.bytes(1);
        compare("C02", &c_library, &rust_library, &values(input, &[]));
    }

    // C03: argc 2, multiple arbitrary non-NUL bytes.
    for _ in 0..CASES_PER_ROW {
        let len = rng.range(2, 129);
        let input = rng.bytes(len);
        compare("C03", &c_library, &rust_library, &values(input, &[]));
    }

    // C04: argc 3, start zero.
    for _ in 0..CASES_PER_ROW {
        let len = rng.range(0, 65);
        let input = rng.bytes(len);
        compare(
            "C04",
            &c_library,
            &rust_library,
            &values(input, &[b"0".to_vec()]),
        );
    }

    // C05: argc 3, interior start.
    for _ in 0..CASES_PER_ROW {
        let len = rng.range(2, 129);
        let start = rng.range(1, len);
        compare(
            "C05",
            &c_library,
            &rust_library,
            &values(rng.bytes(len), &[decimal(start)]),
        );
    }

    // C06: argc 3, start at len.
    for _ in 0..CASES_PER_ROW {
        let len = rng.range(0, 129);
        compare(
            "C06",
            &c_library,
            &rust_library,
            &values(rng.bytes(len), &[decimal(len)]),
        );
    }

    // C07: argc 3, accepted strtol lexical variants.
    for case in 0..CASES_PER_ROW {
        let len = rng.range(1, 129);
        let start = rng.range(0, len + 1);
        let argument = match case % 4 {
            0 => format!("  +{start}tail"),
            1 => format!("\t{start}x"),
            2 => format!("+{start}"),
            _ if start == 0 => "-0suffix".to_owned(),
            _ => format!("{start} suffix"),
        };
        compare(
            "C07",
            &c_library,
            &rust_library,
            &values(rng.bytes(len), &[argument.into_bytes()]),
        );
    }

    // C08: long minimum, nearby cast values, and negative underflow.
    for case in 0..CASES_PER_ROW {
        let len = rng.range(1, 129);
        let argument = if case % 8 == 0 {
            b"-92233720368547758080".to_vec()
        } else {
            let cast_value = rng.range(0, len + 1) as i64;
            (i64::MIN + cast_value).to_string().into_bytes()
        };
        compare(
            "C08",
            &c_library,
            &rust_library,
            &values(rng.bytes(len), &[argument]),
        );
    }

    // C09: argc 4, one-byte slice.
    for _ in 0..CASES_PER_ROW {
        let len = rng.range(1, 129);
        compare(
            "C09",
            &c_library,
            &rust_library,
            &values(rng.bytes(len), &[b"0".to_vec(), b"1".to_vec()]),
        );
    }

    // C10: argc 4, proper prefix.
    for _ in 0..CASES_PER_ROW {
        let len = rng.range(2, 129);
        let stop = rng.range(1, len);
        compare(
            "C10",
            &c_library,
            &rust_library,
            &values(rng.bytes(len), &[b"0".to_vec(), decimal(stop)]),
        );
    }

    // C11: argc 4, interior slice.
    for _ in 0..CASES_PER_ROW {
        let len = rng.range(3, 129);
        let start = rng.range(1, len - 1);
        let stop = rng.range(start + 1, len);
        compare(
            "C11",
            &c_library,
            &rust_library,
            &values(rng.bytes(len), &[decimal(start), decimal(stop)]),
        );
    }

    // C12: argc 4, stop at len.
    for _ in 0..CASES_PER_ROW {
        let len = rng.range(1, 129);
        let start = rng.range(0, len);
        compare(
            "C12",
            &c_library,
            &rust_library,
            &values(rng.bytes(len), &[decimal(start), decimal(len)]),
        );
    }

    // C13: argc 4, accepted lexical variants at both indices.
    for _ in 0..CASES_PER_ROW {
        let len = rng.range(2, 129);
        let start = rng.range(0, len);
        let stop = rng.range(start + 1, len + 1);
        let start_argument = format!(" \t+{start}start").into_bytes();
        let stop_argument = format!("\n+{stop}stop").into_bytes();
        compare(
            "C13",
            &c_library,
            &rust_library,
            &values(rng.bytes(len), &[start_argument, stop_argument]),
        );
    }

    // E01 and E02: invalid argument counts.
    let usage = b"Error: there should be one to three arguments passed:\n\
                  <string> [start] [stop]\n";
    expect_error(
        "E01",
        &c_library,
        &rust_library,
        &[b"driver".to_vec()],
        usage,
    );
    for argc in 5..=16 {
        let arguments = (0..argc).map(|_| b"x".to_vec()).collect::<Vec<_>>();
        expect_error("E02", &c_library, &rust_library, &arguments, usage);
    }

    // E03: second argument has no digits.
    for argument in [
        b"".as_slice(),
        b" ".as_slice(),
        b"\t\n".as_slice(),
        b"+".as_slice(),
        b"-".as_slice(),
        b"words".as_slice(),
    ] {
        expect_error(
            "E03",
            &c_library,
            &rust_library,
            &values(rng.bytes(16), &[argument.to_vec()]),
            b"Second argument must be an integer!",
        );
    }

    // E04: positive one-past/larger, negative, and positive long overflow.
    for case in 0..CASES_PER_ROW {
        let len = rng.range(0, 129);
        let argument = match case % 4 {
            0 => decimal(len + 1),
            1 => decimal(len + rng.range(1, 1024)),
            2 => format!("-{}", rng.range(1, 1024)).into_bytes(),
            _ => b"999999999999999999999999999999999999".to_vec(),
        };
        expect_error(
            "E04",
            &c_library,
            &rust_library,
            &values(rng.bytes(len), &[argument]),
            b"Error: start is off the end of the string!\n",
        );
    }

    // E05: argv[3] aliases the saved end pointer into argv[2].
    compare_alias_error("E05", &c_library, &rust_library);

    // E06: stop is one-past/larger, negative, or positive long overflow.
    for case in 0..CASES_PER_ROW {
        let len = rng.range(1, 129);
        let argument = match case % 4 {
            0 => decimal(len + 1),
            1 => decimal(len + rng.range(1, 1024)),
            2 => format!("-{}", rng.range(1, 1024)).into_bytes(),
            _ => b"999999999999999999999999999999999999".to_vec(),
        };
        expect_error(
            "E06",
            &c_library,
            &rust_library,
            &values(rng.bytes(len), &[b"0".to_vec(), argument]),
            b"Error: stop is off the end of the string!\n",
        );
    }

    // E07: stop is equal/lower, nonnumeric, or negative-long underflow to zero.
    for case in 0..CASES_PER_ROW {
        let len = rng.range(1, 129);
        let start = rng.range(0, len);
        let argument = match case % 4 {
            0 => decimal(start),
            1 if start > 0 => decimal(rng.range(0, start + 1)),
            1 => b"0".to_vec(),
            2 => b"not-a-number".to_vec(),
            _ => b"-99999999999999999999999999999999999".to_vec(),
        };
        expect_error(
            "E07",
            &c_library,
            &rust_library,
            &values(rng.bytes(len), &[decimal(start), argument]),
            b"Error: stop must come after start!\n",
        );
    }

    compare_null_pointer_process_results(&c_path, &rust_path);
}

fn helper_status(library: &Path, case: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "null_pointer_helper",
            "--test-threads=1",
        ])
        .env("DRIVER_NULL_LIBRARY", library)
        .env("DRIVER_NULL_CASE", case)
        .status()
        .unwrap()
}

fn compare_null_pointer_process_results(c_library: &Path, rust_library: &Path) {
    for case in ["argv", "argv1", "argv2", "argv3"] {
        let c = helper_status(c_library, case);
        let rust = helper_status(rust_library, case);
        assert_eq!(c.signal(), Some(11), "C null case {case}: {c:?}");
        assert_eq!(rust.signal(), c.signal(), "Rust null case {case}: {rust:?}");
    }
}

#[test]
#[ignore = "invoked in an isolated subprocess by differential_surface"]
fn null_pointer_helper() {
    let Ok(path) = std::env::var("DRIVER_NULL_LIBRARY") else {
        return;
    };
    let case = std::env::var("DRIVER_NULL_CASE").unwrap();
    let library = unsafe { Library::new(path).unwrap() };
    let function: Symbol<DriverMain> = unsafe { library.get(b"main").unwrap() };

    let mut arguments = match case.as_str() {
        "argv" => {
            unsafe { function(2, ptr::null_mut()) };
            return;
        }
        "argv1" => Arguments::new(&[b"driver".to_vec(), b"x".to_vec()]),
        "argv2" => Arguments::new(&[b"driver".to_vec(), b"x".to_vec(), b"0".to_vec()]),
        "argv3" => Arguments::new(&[
            b"driver".to_vec(),
            b"x".to_vec(),
            b"0".to_vec(),
            b"1".to_vec(),
        ]),
        _ => panic!("unknown null case"),
    };

    match case.as_str() {
        "argv1" => arguments.pointers[1] = ptr::null_mut(),
        "argv2" => arguments.pointers[2] = ptr::null_mut(),
        "argv3" => arguments.pointers[3] = ptr::null_mut(),
        _ => unreachable!(),
    }
    unsafe { function(arguments.argc(), arguments.pointers.as_mut_ptr()) };
}
