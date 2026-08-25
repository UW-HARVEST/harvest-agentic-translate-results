use libloading::Library;
use std::ffi::{c_char, c_int, c_void, CString};
use std::path::PathBuf;
use std::ptr;
use std::sync::{Mutex, MutexGuard};

type StaticSumFn = unsafe extern "C" fn(c_int) -> c_int;
type MainFn = unsafe extern "C" fn(c_int, *mut *mut c_char) -> c_int;

struct Api {
    _library: Library,
    static_sum: StaticSumFn,
    main: MainFn,
}

struct Pair {
    c: Api,
    rust: Api,
}

#[derive(Debug, PartialEq, Eq)]
struct MainResult {
    status: c_int,
    stdout: Vec<u8>,
}

static TEST_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

fn serial() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

fn c_library_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libdriver_c.so")
}

fn rust_library_path() -> PathBuf {
    let executable = std::env::current_exe().expect("resolve integration-test executable");
    executable
        .parent()
        .expect("integration-test deps directory")
        .join("libdriver.so")
}

unsafe fn load_api(path: PathBuf) -> Api {
    assert!(
        path.is_file(),
        "shared library is missing: {}",
        path.display()
    );
    let library = unsafe { Library::new(&path) }
        .unwrap_or_else(|error| panic!("load {}: {error}", path.display()));
    let static_sum = unsafe {
        *library
            .get::<StaticSumFn>(b"static_sum\0")
            .unwrap_or_else(|error| panic!("load static_sum from {}: {error}", path.display()))
    };
    let main = unsafe {
        *library
            .get::<MainFn>(b"main\0")
            .unwrap_or_else(|error| panic!("load main from {}: {error}", path.display()))
    };
    Api {
        _library: library,
        static_sum,
        main,
    }
}

fn load_pair() -> Pair {
    unsafe {
        Pair {
            c: load_api(c_library_path()),
            rust: load_api(rust_library_path()),
        }
    }
}

fn capture_stdout<T>(call: impl FnOnce() -> T) -> (T, Vec<u8>) {
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "flush stdout before capture");

        let mut pipe_fds = [-1, -1];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "create stdout pipe");
        let saved_stdout = dup(1);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(dup2(pipe_fds[1], 1), 1, "redirect stdout");
        assert_eq!(close(pipe_fds[1]), 0, "close duplicate pipe writer");

        let result = call();

        assert_eq!(fflush(ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(dup2(saved_stdout, 1), 1, "restore stdout");
        assert_eq!(close(saved_stdout), 0, "close saved stdout");

        let mut output = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = read(
                pipe_fds[0],
                buffer.as_mut_ptr().cast::<c_void>(),
                buffer.len(),
            );
            assert!(count >= 0, "read captured stdout");
            if count == 0 {
                break;
            }
            output.extend_from_slice(&buffer[..count as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0, "close pipe reader");
        (result, output)
    }
}

fn call_main(api: &Api, argument: &str) -> MainResult {
    let program = CString::new("driver").unwrap();
    let argument = CString::new(argument).expect("test argument contains no NUL");
    let mut argv = [
        program.as_ptr().cast_mut(),
        argument.as_ptr().cast_mut(),
        ptr::null_mut(),
    ];
    let (status, stdout) = capture_stdout(|| unsafe { (api.main)(2, argv.as_mut_ptr()) });
    MainResult { status, stdout }
}

fn call_main_with_argc(api: &Api, argc: c_int, null_argv: bool) -> MainResult {
    let program = CString::new("driver").unwrap();
    let argument = CString::new("1").unwrap();
    let mut argv = [
        program.as_ptr().cast_mut(),
        argument.as_ptr().cast_mut(),
        ptr::null_mut(),
    ];
    let argv = if null_argv {
        ptr::null_mut()
    } else {
        argv.as_mut_ptr()
    };
    let (status, stdout) = capture_stdout(|| unsafe { (api.main)(argc, argv) });
    MainResult { status, stdout }
}

fn assert_main_equal(pair: &Pair, argument: &str, row: usize) {
    let c = call_main(&pair.c, argument);
    let rust = call_main(&pair.rust, argument);
    assert_eq!(rust, c, "CONFIGS.md row {row}, argument {argument:?}");
}

fn child_signal(main: MainFn, null_argv: bool) -> c_int {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork crash-boundary child");
        if pid == 0 {
            if null_argv {
                main(2, ptr::null_mut());
            } else {
                let mut argv = [ptr::null_mut(), ptr::null_mut()];
                main(2, argv.as_mut_ptr());
            }
            _exit(0);
        }

        let mut status = 0;
        assert_eq!(waitpid(pid, &mut status, 0), pid, "wait for child");
        status & 0x7f
    }
}

struct Lcg(u64);

impl Lcg {
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

    fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }

    fn bounded_i32(&mut self, magnitude: i32) -> i32 {
        self.next_i32() % magnitude
    }
}

#[test]
fn exported_symbol_surface_loads_from_both_shared_objects() {
    let _guard = serial();
    let pair = load_pair();
    let _ = pair.c.static_sum;
    let _ = pair.c.main;
    let _ = pair.rust.static_sum;
    let _ = pair.rust.main;
}

#[test]
fn all_configuration_rows_match_for_randomized_inputs() {
    let _guard = serial();
    let pair = load_pair();
    let mut rng = Lcg::new(0x5eed_c0de_d15c_a11);

    // CONFIGS.md row 1: direct low-level state transitions.
    let mut updates = vec![0, 1, -1, c_int::MAX, c_int::MIN];
    updates.extend((0..256).map(|_| rng.next_i32()));
    for update in updates {
        let c = unsafe { (pair.c.static_sum)(update) };
        let rust = unsafe { (pair.rust.static_sum)(update) };
        assert_eq!(rust, c, "CONFIGS.md row 1, update {update}");
    }

    // CONFIGS.md row 2: canonical unsigned decimal text.
    assert_main_equal(&pair, "0", 2);
    for _ in 0..128 {
        let stride = (rng.next_u64() % 1_000_001) as u32;
        assert_main_equal(&pair, &stride.to_string(), 2);
    }

    // CONFIGS.md row 3: C whitespace plus optional signs.
    for index in 0..128 {
        let stride = rng.bounded_i32(1_000_000);
        let argument = match index % 4 {
            0 => format!(" \t+{}", stride.unsigned_abs()),
            1 => format!("\n  -{}", stride.unsigned_abs()),
            2 => format!("+{}", stride.unsigned_abs()),
            _ => stride.to_string(),
        };
        assert_main_equal(&pair, &argument, 3);
    }

    // CONFIGS.md row 4: a converted prefix followed by ignored suffix text.
    for index in 0..128 {
        let stride = rng.bounded_i32(1_000_000);
        let suffix = match index % 4 {
            0 => "xyz",
            1 => " ",
            2 => ".75",
            _ => "e100",
        };
        assert_main_equal(&pair, &format!("{stride}{suffix}"), 4);
    }

    // CONFIGS.md row 5: int/long boundaries and strtol saturation.
    let boundaries = [
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "999999999999999999999999999999999999999999999999",
        "-999999999999999999999999999999999999999999999999",
    ];
    for argument in boundaries {
        assert_main_equal(&pair, argument, 5);
    }
    for _ in 0..128 {
        let int_value = rng.next_i32();
        assert_main_equal(&pair, &int_value.to_string(), 5);

        let long_value = rng.next_u64() as i64;
        assert_main_equal(&pair, &long_value.to_string(), 5);

        let first_digit = (rng.next_u64() % 9) + 1;
        let huge = format!(
            "{first_digit}{:019}{:019}",
            rng.next_u64() % 10_000_000_000_000_000_000,
            rng.next_u64() % 10_000_000_000_000_000_000
        );
        assert_main_equal(&pair, &huge, 5);
    }

    // CONFIGS.md row 6: mixed direct and top-level calls share static state.
    for index in 0..128 {
        let update = rng.next_i32();
        let c = unsafe { (pair.c.static_sum)(update) };
        let rust = unsafe { (pair.rust.static_sum)(update) };
        assert_eq!(rust, c, "CONFIGS.md row 6 direct call {index}");

        let stride = rng.bounded_i32(10_000);
        assert_main_equal(&pair, &stride.to_string(), 6);
    }
}

#[test]
fn all_error_rows_and_generic_boundaries_match() {
    let _guard = serial();
    let pair = load_pair();

    // ERRORS.md row 1, including null argv and argc boundaries.
    let expected_argc_error = MainResult {
        status: 1,
        stdout: b"Error: should only be a single (integer) argument!\n".to_vec(),
    };
    for argc in [c_int::MIN, -1, 0, 1, 3, c_int::MAX] {
        for null_argv in [false, true] {
            let c = call_main_with_argc(&pair.c, argc, null_argv);
            let rust = call_main_with_argc(&pair.rust, argc, null_argv);
            assert_eq!(
                rust, c,
                "ERRORS.md row 1, argc {argc}, null argv {null_argv}"
            );
            assert_eq!(c, expected_argc_error, "C ground truth for ERRORS.md row 1");
        }
    }

    // ERRORS.md row 2: every input consumes zero characters.
    let expected_parse_error = MainResult {
        status: 1,
        stdout: b"Error: first argument must be an integer!\n".to_vec(),
    };
    for argument in ["", "abc", "   ", "\t\nx", "+", "-", " +x", "invalid123"] {
        let c = call_main(&pair.c, argument);
        let rust = call_main(&pair.rust, argument);
        assert_eq!(rust, c, "ERRORS.md row 2, argument {argument:?}");
        assert_eq!(
            c, expected_parse_error,
            "C ground truth for ERRORS.md row 2"
        );
    }

    // Neither implementation checks these pointers when argc is valid. Compare
    // the externally visible process signal in isolated children.
    for null_argv in [true, false] {
        let c_signal = child_signal(pair.c.main, null_argv);
        let rust_signal = child_signal(pair.rust.main, null_argv);
        assert_eq!(
            rust_signal, c_signal,
            "generic null boundary, null argv {null_argv}"
        );
        assert_eq!(c_signal, 11, "C ground truth is SIGSEGV");
    }
}
