use libloading::Library;
use std::env;
use std::ffi::{c_char, c_int, c_void, CString};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;

type PrintLine = unsafe extern "C" fn(*const c_char);
type PrintIntLine = unsafe extern "C" fn(c_int);
type NoArgs = unsafe extern "C" fn();
type Main = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
const RANDOM_CASES: usize = 256;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

struct Api {
    _library: Library,
    print_line: PrintLine,
    print_int_line: PrintIntLine,
    bad: NoArgs,
    good: NoArgs,
    main: Main,
}

impl Api {
    unsafe fn load(path: PathBuf) -> Self {
        let library = Library::new(&path)
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let print_line = *library
            .get::<PrintLine>(b"printLine\0")
            .expect("missing printLine");
        let print_int_line = *library
            .get::<PrintIntLine>(b"printIntLine\0")
            .expect("missing printIntLine");
        let bad = *library.get::<NoArgs>(b"bad\0").expect("missing bad");
        let good = *library.get::<NoArgs>(b"good\0").expect("missing good");
        let main = *library.get::<Main>(b"main\0").expect("missing main");

        Self {
            _library: library,
            print_line,
            print_int_line,
            bad,
            good,
            main,
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("c_src/build/libdriver_c.so")
}

fn rust_library_path() -> PathBuf {
    let executable = env::current_exe().expect("test executable path");
    let profile_dir = executable
        .parent()
        .and_then(|deps| deps.parent())
        .expect("Cargo profile directory");
    profile_dir.join("libdriver.so")
}

fn load_apis() -> (Api, Api) {
    unsafe { (Api::load(c_library_path()), Api::load(rust_library_path())) }
}

fn capture_stdout<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _guard = STDOUT_LOCK.lock().expect("stdout lock");

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "flush before capture");

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "create stdout pipe");
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "redirect stdout"
        );
        assert_eq!(close(pipe_fds[1]), 0, "close duplicated write end");

        let result = operation();

        assert_eq!(fflush(ptr::null_mut()), 0, "flush captured output");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "close saved stdout");

        let mut output = Vec::new();
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .expect("read captured stdout");
        (result, output)
    }
}

fn outputs<T>(
    c_operation: impl FnOnce() -> T,
    rust_operation: impl FnOnce() -> T,
) -> ((T, Vec<u8>), (T, Vec<u8>)) {
    (capture_stdout(c_operation), capture_stdout(rust_operation))
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

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
}

fn random_c_strings() -> Vec<CString> {
    let mut rng = Rng::new(0x7a63_61f0_8d42_b195);
    let mut strings = vec![CString::new(Vec::<u8>::new()).unwrap()];

    for _ in 1..RANDOM_CASES {
        let length = (rng.next_u64() % 257) as usize;
        let bytes = (0..length)
            .map(|_| (rng.next_u64() % 255 + 1) as u8)
            .collect::<Vec<_>>();
        strings.push(CString::new(bytes).unwrap());
    }
    strings
}

fn all_c_symbols_are_loadable_from_both_shared_objects() {
    let _apis = load_apis();
}

fn config_1_print_line_matches_for_randomized_strings() {
    let (c_api, rust_api) = load_apis();
    let strings = random_c_strings();

    let (((), c_output), ((), rust_output)) = outputs(
        || unsafe {
            for string in &strings {
                (c_api.print_line)(string.as_ptr());
            }
        },
        || unsafe {
            for string in &strings {
                (rust_api.print_line)(string.as_ptr());
            }
        },
    );

    assert_eq!(rust_output, c_output);
}

fn config_2_print_int_line_matches_for_randomized_integers() {
    let (c_api, rust_api) = load_apis();
    let mut values = vec![i32::MIN, -1, 0, 1, i32::MAX];
    let mut rng = Rng::new(0x19dc_e82b_6037_4a51);
    values.extend((values.len()..RANDOM_CASES).map(|_| rng.next_i32()));

    let (((), c_output), ((), rust_output)) = outputs(
        || unsafe {
            for value in &values {
                (c_api.print_int_line)(*value);
            }
        },
        || unsafe {
            for value in &values {
                (rust_api.print_int_line)(*value);
            }
        },
    );

    assert_eq!(rust_output, c_output);
}

fn config_3_bad_matches() {
    let (c_api, rust_api) = load_apis();
    let (((), c_output), ((), rust_output)) =
        outputs(|| unsafe { (c_api.bad)() }, || unsafe { (rust_api.bad)() });

    assert_eq!(rust_output, c_output);
}

fn config_4_good_matches() {
    let (c_api, rust_api) = load_apis();
    let (((), c_output), ((), rust_output)) = outputs(
        || unsafe { (c_api.good)() },
        || unsafe { (rust_api.good)() },
    );

    assert_eq!(rust_output, c_output);
}

fn config_5_main_matches_for_randomized_ignored_arguments() {
    let (c_api, rust_api) = load_apis();
    let arg0 = CString::new("driver").unwrap();
    let mut argv = [arg0.as_ptr().cast_mut(), ptr::null_mut()];
    let mut rng = Rng::new(0xd602_99a1_47fe_35c8);
    let argc_values = (0..RANDOM_CASES)
        .map(|_| rng.next_i32())
        .collect::<Vec<_>>();

    let (c_result, c_output) = capture_stdout(|| unsafe {
        argc_values
            .iter()
            .enumerate()
            .map(|(index, argc)| {
                let argv = if index % 2 == 0 {
                    argv.as_mut_ptr()
                } else {
                    ptr::null_mut()
                };
                (c_api.main)(*argc, argv)
            })
            .collect::<Vec<_>>()
    });
    let (rust_result, rust_output) = capture_stdout(|| unsafe {
        argc_values
            .iter()
            .enumerate()
            .map(|(index, argc)| {
                let argv = if index % 2 == 0 {
                    argv.as_mut_ptr()
                } else {
                    ptr::null_mut()
                };
                (rust_api.main)(*argc, argv)
            })
            .collect::<Vec<_>>()
    });

    assert_eq!(rust_result, c_result);
    assert_eq!(rust_output, c_output);
}

fn error_1_print_line_null_matches() {
    let (c_api, rust_api) = load_apis();
    let (((), c_output), ((), rust_output)) = outputs(
        || unsafe { (c_api.print_line)(ptr::null()) },
        || unsafe { (rust_api.print_line)(ptr::null()) },
    );

    assert_eq!(rust_output, c_output);
    assert!(c_output.is_empty());
}

fn generic_main_boundaries_with_null_argv_match() {
    let (c_api, rust_api) = load_apis();
    let argc_values = [i32::MIN, -1, 0, 1, i32::MAX];

    for argc in argc_values {
        let ((c_result, c_output), (rust_result, rust_output)) = outputs(
            || unsafe { (c_api.main)(argc, ptr::null_mut()) },
            || unsafe { (rust_api.main)(argc, ptr::null_mut()) },
        );
        assert_eq!(rust_result, c_result, "argc {argc}");
        assert_eq!(rust_output, c_output, "argc {argc}");
    }
}

#[test]
fn differential_surface_matches() {
    all_c_symbols_are_loadable_from_both_shared_objects();
    config_1_print_line_matches_for_randomized_strings();
    config_2_print_int_line_matches_for_randomized_integers();
    config_3_bad_matches();
    config_4_good_matches();
    config_5_main_matches_for_randomized_ignored_arguments();
    error_1_print_line_null_matches();
    generic_main_boundaries_with_null_argv_match();
}
