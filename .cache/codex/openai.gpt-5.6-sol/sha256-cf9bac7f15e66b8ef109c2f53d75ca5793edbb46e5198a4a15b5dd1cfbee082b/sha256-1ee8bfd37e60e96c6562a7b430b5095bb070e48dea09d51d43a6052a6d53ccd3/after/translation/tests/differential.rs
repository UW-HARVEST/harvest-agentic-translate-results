use libloading::Library;
use std::env;
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;

type Cleanup = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type CleanupResources = unsafe extern "C" fn(*mut c_char);
type PrintResult = unsafe extern "C" fn(*const c_char, c_int);
type ArmFailure = unsafe extern "C" fn();

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

const TRIALS: usize = 32;
const STDOUT_FILENO: c_int = 1;

struct Api {
    _library: Library,
    cleanup: Cleanup,
    cleanup_resources: CleanupResources,
    print_result: PrintResult,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        // SAFETY: The paths name the two freshly built libraries under test.
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        // SAFETY: Each symbol type is copied directly from the C header.
        let cleanup = unsafe { *library.get::<Cleanup>(b"cleanup\0").unwrap() };
        // SAFETY: Each symbol type is copied directly from the C implementation.
        let cleanup_resources = unsafe {
            *library
                .get::<CleanupResources>(b"cleanup_resources\0")
                .unwrap()
        };
        // SAFETY: Each symbol type is copied directly from the C implementation.
        let print_result = unsafe { *library.get::<PrintResult>(b"print_result\0").unwrap() };
        Self {
            _library: library,
            cleanup,
            cleanup_resources,
            print_result,
        }
    }
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

    fn nonzero_byte(&mut self) -> u8 {
        (self.next_u64() % 255 + 1) as u8
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        crate_root.join("../c_src/build/libharvest-work-KQ5axu.so"),
        crate_root.join("target/release/libcleanup_lib.so"),
    )
}

fn load_apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing release Rust library: {}; run cargo build --release",
        rust_path.display()
    );
    // SAFETY: Api::load validates that every required symbol can be resolved.
    unsafe { (Api::load(&c_path), Api::load(&rust_path)) }
}

fn capture_stdout<T>(operation: impl FnOnce() -> T) -> (T, Vec<u8>) {
    // SAFETY: Every acquired descriptor is checked and closed exactly once.
    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0);
        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0);
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0);
        assert_eq!(dup2(pipe_fds[1], STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(pipe_fds[1]), 0);

        let result = operation();

        assert_eq!(fflush(ptr::null_mut()), 0);
        assert_eq!(dup2(saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
        assert_eq!(close(saved_stdout), 0);

        let mut output = Vec::new();
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .unwrap();
        (result, output)
    }
}

fn invoke_cleanup(api: &Api, inputs: &[[i32; 4]]) -> (Vec<i32>, Vec<u8>) {
    capture_stdout(|| {
        inputs
            .iter()
            .map(|values| {
                // SAFETY: cleanup takes four integers and has no pointer preconditions.
                unsafe { (api.cleanup)(values[0], values[1], values[2], values[3]) }
            })
            .collect()
    })
}

fn invoke_print_result(api: &Api, labels: &[Vec<u8>], results: &[i32]) -> Vec<u8> {
    assert_eq!(labels.len(), results.len());
    let ((), output) = capture_stdout(|| {
        for (label, result) in labels.iter().zip(results) {
            assert_eq!(label.last(), Some(&0));
            // SAFETY: Each byte vector remains live and is NUL-terminated.
            unsafe { (api.print_result)(label.as_ptr().cast(), *result) };
        }
    });
    output
}

fn random_other(rng: &mut Rng) -> i32 {
    loop {
        let value = rng.next_i32();
        if ![10, 20, 30, 40].contains(&value) {
            return value;
        }
    }
}

fn value_for_class(class: usize, rng: &mut Rng) -> i32 {
    match class {
        0 => 10,
        1 => 20,
        2 => 30,
        3 => 40,
        4 => random_other(rng),
        _ => unreachable!(),
    }
}

fn configs_cleanup_rows_1_through_625() {
    let (c_api, rust_api) = load_apis();
    let mut row = 0usize;

    for a_class in 0..5 {
        for b_class in 0..5 {
            for c_class in 0..5 {
                for d_class in 0..5 {
                    row += 1;
                    let mut rng = Rng::new(0x6a09_e667_f3bc_c909 ^ row as u64);
                    let inputs: Vec<_> = (0..TRIALS)
                        .map(|_| {
                            [
                                value_for_class(a_class, &mut rng),
                                value_for_class(b_class, &mut rng),
                                value_for_class(c_class, &mut rng),
                                value_for_class(d_class, &mut rng),
                            ]
                        })
                        .collect();

                    let (c_results, c_output) = invoke_cleanup(&c_api, &inputs);
                    let (rust_results, rust_output) = invoke_cleanup(&rust_api, &inputs);
                    assert_eq!(rust_results, c_results, "CONFIGS.md row {row} results");
                    assert_eq!(rust_output, c_output, "CONFIGS.md row {row} stdout");
                }
            }
        }
    }
    assert_eq!(row, 625);
}

fn configs_print_result_rows_626_through_629() {
    let (c_api, rust_api) = load_apis();
    let mut rng = Rng::new(0xbb67_ae85_84ca_a73b);

    let empty_labels = vec![vec![0]; TRIALS];
    let empty_results: Vec<_> = (0..TRIALS).map(|_| rng.next_i32()).collect();
    assert_eq!(
        invoke_print_result(&rust_api, &empty_labels, &empty_results),
        invoke_print_result(&c_api, &empty_labels, &empty_results),
        "CONFIGS.md row 626"
    );

    let arbitrary_labels: Vec<_> = (0..TRIALS)
        .map(|_| {
            let length = (rng.next_u64() % 128 + 1) as usize;
            let mut label: Vec<_> = (0..length).map(|_| rng.nonzero_byte()).collect();
            label.push(0);
            label
        })
        .collect();
    let arbitrary_results: Vec<_> = (0..TRIALS).map(|_| rng.next_i32()).collect();
    assert_eq!(
        invoke_print_result(&rust_api, &arbitrary_labels, &arbitrary_results),
        invoke_print_result(&c_api, &arbitrary_labels, &arbitrary_results),
        "CONFIGS.md row 627"
    );

    let embedded_nul_labels: Vec<_> = (0..TRIALS)
        .map(|_| {
            let prefix_length = (rng.next_u64() % 32) as usize;
            let suffix_length = (rng.next_u64() % 32 + 1) as usize;
            let mut label: Vec<_> = (0..prefix_length).map(|_| rng.nonzero_byte()).collect();
            label.push(0);
            label.extend((0..suffix_length).map(|_| rng.nonzero_byte()));
            label.push(0);
            label
        })
        .collect();
    let embedded_nul_results: Vec<_> = (0..TRIALS).map(|_| rng.next_i32()).collect();
    assert_eq!(
        invoke_print_result(&rust_api, &embedded_nul_labels, &embedded_nul_results),
        invoke_print_result(&c_api, &embedded_nul_labels, &embedded_nul_results),
        "CONFIGS.md row 628"
    );

    let oversized_labels: Vec<_> = (0..8)
        .map(|_| {
            let mut label: Vec<_> = (0..4096).map(|_| rng.nonzero_byte()).collect();
            label.push(0);
            label
        })
        .collect();
    let oversized_results = vec![i32::MIN, -1, 0, 1, i32::MAX, i32::MIN, 0, i32::MAX];
    assert_eq!(
        invoke_print_result(&rust_api, &oversized_labels, &oversized_results),
        invoke_print_result(&c_api, &oversized_labels, &oversized_results),
        "CONFIGS.md row 629"
    );
}

fn configs_cleanup_resources_rows_630_and_631() {
    let (c_api, rust_api) = load_apis();

    let ((), c_null_output) =
        capture_stdout(|| unsafe { (c_api.cleanup_resources)(ptr::null_mut()) });
    let ((), rust_null_output) =
        capture_stdout(|| unsafe { (rust_api.cleanup_resources)(ptr::null_mut()) });
    assert_eq!(rust_null_output, c_null_output, "CONFIGS.md row 630");

    let mut rng = Rng::new(0x3c6e_f372_fe94_f82b);
    let sizes: Vec<_> = (0..TRIALS)
        .map(|_| (rng.next_u64() % 8192 + 1) as usize)
        .collect();
    let free_with = |api: &Api| {
        let ((), output) = capture_stdout(|| {
            for size in &sizes {
                // SAFETY: malloc returns a free-compatible pointer for cleanup_resources.
                let allocation = unsafe { malloc(*size) }.cast::<c_char>();
                assert!(!allocation.is_null());
                // SAFETY: Ownership of this allocation is transferred exactly once.
                unsafe { (api.cleanup_resources)(allocation) };
            }
        });
        output
    };
    assert_eq!(
        free_with(&rust_api),
        free_with(&c_api),
        "CONFIGS.md row 631"
    );
}

fn run_preloaded_test(error_case: &str) {
    if env::var_os("DIFFERENTIAL_PRELOADED").is_some() {
        return;
    }

    let interposer = env!("FAILURE_INTERPOSER");
    let preload = match env::var_os("LD_PRELOAD") {
        Some(existing) if !existing.is_empty() => {
            format!("{interposer}:{}", existing.to_string_lossy())
        }
        _ => interposer.to_owned(),
    };
    let output = Command::new(env::current_exe().unwrap())
        .args(["--exact", "differential_all_rows", "--nocapture"])
        .env("DIFFERENTIAL_PRELOADED", "1")
        .env("DIFFERENTIAL_ERROR_CASE", error_case)
        .env("LD_PRELOAD", preload)
        .output()
        .expect("failed to run preloaded differential test");
    assert!(
        output.status.success(),
        "preloaded error case {error_case} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn load_failure_arm(symbol: &[u8]) -> (Library, ArmFailure) {
    // SAFETY: The build script produced this test-only shared object.
    let library = unsafe { Library::new(env!("FAILURE_INTERPOSER")) }.unwrap();
    // SAFETY: Both exported arm functions have the same no-argument signature.
    let arm = unsafe { *library.get::<ArmFailure>(symbol).unwrap() };
    (library, arm)
}

fn errors_validation_row_1() {
    if env::var_os("DIFFERENTIAL_PRELOADED").is_none() {
        run_preloaded_test("validation");
        return;
    }

    let (c_api, rust_api) = load_apis();
    let (_interposer, arm) = load_failure_arm(b"fail_next_strncmp\0");
    let input = [10, 20, 30, 40];

    let (c_result, c_output) = capture_stdout(|| unsafe {
        arm();
        (c_api.cleanup)(input[0], input[1], input[2], input[3])
    });
    let (rust_result, rust_output) = capture_stdout(|| unsafe {
        arm();
        (rust_api.cleanup)(input[0], input[1], input[2], input[3])
    });
    assert_eq!(c_result, 0);
    assert_eq!(rust_result, c_result, "ERRORS.md row 1 result");
    assert_eq!(c_output, b"Input string validation failed.\n");
    assert_eq!(rust_output, c_output, "ERRORS.md row 1 stdout");
}

fn errors_malloc_failure_row_2() {
    if env::var_os("DIFFERENTIAL_PRELOADED").is_none() {
        run_preloaded_test("malloc");
        return;
    }

    let (c_api, rust_api) = load_apis();
    let (_interposer, arm) = load_failure_arm(b"fail_next_malloc_50\0");
    let input = [10, 20, 30, 40];

    let (c_result, c_output) = capture_stdout(|| unsafe {
        arm();
        (c_api.cleanup)(input[0], input[1], input[2], input[3])
    });
    let (rust_result, rust_output) = capture_stdout(|| unsafe {
        arm();
        (rust_api.cleanup)(input[0], input[1], input[2], input[3])
    });
    assert_eq!(c_result, 160);
    assert_eq!(rust_result, c_result, "ERRORS.md row 2 result");
    assert_eq!(c_output, b"Memory allocation failed.\n");
    assert_eq!(rust_output, c_output, "ERRORS.md row 2 stdout");
}

fn errors_null_pointer_rows_3_and_4() {
    let (c_api, rust_api) = load_apis();

    let ((), c_print_output) = capture_stdout(|| unsafe { (c_api.print_result)(ptr::null(), -17) });
    let ((), rust_print_output) =
        capture_stdout(|| unsafe { (rust_api.print_result)(ptr::null(), -17) });
    assert_eq!(c_print_output, b"(null): -17\n");
    assert_eq!(rust_print_output, c_print_output, "ERRORS.md row 3");

    let ((), c_cleanup_output) =
        capture_stdout(|| unsafe { (c_api.cleanup_resources)(ptr::null_mut()) });
    let ((), rust_cleanup_output) =
        capture_stdout(|| unsafe { (rust_api.cleanup_resources)(ptr::null_mut()) });
    assert_eq!(c_cleanup_output, b"");
    assert_eq!(rust_cleanup_output, c_cleanup_output, "ERRORS.md row 4");
}

#[test]
fn differential_all_rows() {
    match env::var("DIFFERENTIAL_ERROR_CASE").as_deref() {
        Ok("validation") => errors_validation_row_1(),
        Ok("malloc") => errors_malloc_failure_row_2(),
        Ok(other) => panic!("unknown differential error case: {other}"),
        Err(_) => {
            configs_cleanup_rows_1_through_625();
            configs_print_result_rows_626_through_629();
            configs_cleanup_resources_rows_630_and_631();
            errors_validation_row_1();
            errors_malloc_failure_row_2();
            errors_null_pointer_rows_3_and_4();
        }
    }
}
