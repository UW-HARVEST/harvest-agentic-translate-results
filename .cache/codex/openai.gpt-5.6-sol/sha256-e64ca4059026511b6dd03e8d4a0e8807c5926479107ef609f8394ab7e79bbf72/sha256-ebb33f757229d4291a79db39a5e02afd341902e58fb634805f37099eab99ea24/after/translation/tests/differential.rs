use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

type Bin2Hex = unsafe extern "C" fn(*mut c_char, usize, *const u8, usize) -> *mut c_char;

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    fn load() -> Self {
        let c_path = c_library_path();
        let rust_path = rust_library_path();
        assert!(
            c_path.is_file(),
            "C shared library is missing: {}",
            c_path.display()
        );
        assert!(
            rust_path.is_file(),
            "Rust shared library is missing: {}",
            rust_path.display()
        );

        Self {
            // SAFETY: Both paths identify shared libraries built by this project.
            c: unsafe { Library::new(c_path).expect("load C shared library") },
            // SAFETY: Both paths identify shared libraries built by this project.
            rust: unsafe { Library::new(rust_path).expect("load Rust shared library") },
        }
    }

    unsafe fn invoke(
        library: &Library,
        output: *mut c_char,
        output_capacity: usize,
        input: *const u8,
        input_len: usize,
    ) -> *mut c_char {
        // SAFETY: The symbol name and signature come directly from the C header.
        let function: Symbol<'_, Bin2Hex> =
            unsafe { library.get(b"bin2hex\0").expect("resolve bin2hex") };
        // SAFETY: Each caller constructs pointers and lengths for its test case.
        unsafe { function(output, output_capacity, input, input_len) }
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

    fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }

    fn usize_inclusive(&mut self, minimum: usize, maximum: usize) -> usize {
        minimum + self.next_u64() as usize % (maximum - minimum + 1)
    }

    fn fill(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.byte();
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation crate has a workspace parent")
        .to_path_buf()
}

fn c_library_path() -> PathBuf {
    let root = workspace_root();
    let project_name = root
        .file_name()
        .expect("workspace root has a file name")
        .to_string_lossy();
    root.join("c_src")
        .join("build")
        .join(format!("lib{project_name}.so"))
}

fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("BIN2HEX_RUST_SO") {
        return PathBuf::from(path);
    }

    let test_executable = std::env::current_exe().expect("locate current test executable");
    let profile_directory = test_executable
        .parent()
        .and_then(Path::parent)
        .expect("test executable is under target/<profile>/deps");
    let profile_library = profile_directory.join("libbin2hex_lib.so");
    if profile_library.is_file() {
        return profile_library;
    }

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("libbin2hex_lib.so")
}

fn compare_success(
    libraries: &Libraries,
    input: *const u8,
    input_len: usize,
    output_capacity: usize,
    initial_output: &[u8],
) {
    let mut c_output = initial_output.to_vec();
    let mut rust_output = initial_output.to_vec();

    // SAFETY: The output allocations are identical and large enough for the valid
    // case. The input pointer is either valid for input_len bytes or input_len is
    // zero, in which case the C implementation does not dereference it.
    let (c_return, rust_return) = unsafe {
        (
            Libraries::invoke(
                &libraries.c,
                c_output.as_mut_ptr().cast(),
                output_capacity,
                input,
                input_len,
            ),
            Libraries::invoke(
                &libraries.rust,
                rust_output.as_mut_ptr().cast(),
                output_capacity,
                input,
                input_len,
            ),
        )
    };

    assert_eq!(
        c_return,
        c_output.as_mut_ptr().cast(),
        "C did not return its output pointer"
    );
    assert_eq!(
        rust_return,
        rust_output.as_mut_ptr().cast(),
        "Rust did not return its output pointer"
    );
    assert_eq!(
        rust_output, c_output,
        "complete output allocation, including canaries, differs"
    );
}

fn randomized_success_case(
    seed: u64,
    iterations: usize,
    length_range: std::ops::RangeInclusive<usize>,
    slack_range: std::ops::RangeInclusive<usize>,
) {
    let libraries = Libraries::load();
    let mut rng = Rng::new(seed);
    for iteration in 0..iterations {
        let input_len = rng.usize_inclusive(*length_range.start(), *length_range.end());
        let slack = rng.usize_inclusive(*slack_range.start(), *slack_range.end());
        let output_capacity = input_len * 2 + 1 + slack;
        let mut input = vec![0_u8; input_len];
        rng.fill(&mut input);

        // Keep randomized bytes beyond the declared capacity as overwrite canaries.
        let mut initial_output = vec![0_u8; output_capacity + 32];
        rng.fill(&mut initial_output);
        compare_success(
            &libraries,
            input.as_ptr(),
            input.len(),
            output_capacity,
            &initial_output,
        );

        assert!(
            iteration < iterations,
            "retain the iteration in assertion diagnostics"
        );
    }
}

#[test]
fn config_01_empty_minimum_capacity() {
    randomized_success_case(0x4c01_5eed_f00d_0001, 256, 0..=0, 0..=0);
}

#[test]
fn config_02_empty_slack_capacity() {
    randomized_success_case(0x4c02_5eed_f00d_0002, 256, 0..=0, 1..=127);
}

#[test]
fn config_03_one_byte_minimum_capacity() {
    randomized_success_case(0x4c03_5eed_f00d_0003, 512, 1..=1, 0..=0);
}

#[test]
fn config_04_one_byte_slack_capacity() {
    randomized_success_case(0x4c04_5eed_f00d_0004, 512, 1..=1, 1..=127);
}

#[test]
fn config_05_many_bytes_minimum_capacity() {
    randomized_success_case(0x4c05_5eed_f00d_0005, 128, 2..=4096, 0..=0);
}

#[test]
fn config_06_many_bytes_slack_capacity() {
    randomized_success_case(0x4c06_5eed_f00d_0006, 128, 2..=4096, 1..=127);
}

#[cfg(unix)]
fn run_death_child(library: &str, case: &str) -> ExitStatus {
    Command::new(std::env::current_exe().expect("locate test executable"))
        .args(["--ignored", "--exact", "ffi_child"])
        .env("BIN2HEX_CHILD_LIBRARY", library)
        .env("BIN2HEX_CHILD_CASE", case)
        .status()
        .expect("run isolated FFI child")
}

#[cfg(unix)]
fn compare_signal(case: &str, expected_signal: i32) {
    use std::os::unix::process::ExitStatusExt;

    let c_status = run_death_child("c", case);
    let rust_status = run_death_child("rust", case);
    assert_eq!(
        c_status.signal(),
        Some(expected_signal),
        "unexpected C termination for {case}: {c_status:?}"
    );
    assert_eq!(
        rust_status.signal(),
        c_status.signal(),
        "Rust termination differs from C for {case}: C={c_status:?}, Rust={rust_status:?}"
    );
}

#[cfg(unix)]
#[test]
fn error_01_oversized_length_threshold_aborts() {
    compare_signal("oversized_threshold", 6);
}

#[cfg(unix)]
#[test]
fn error_02_capacity_equal_to_required_bytes_aborts() {
    compare_signal("capacity_equal", 6);
}

#[cfg(unix)]
#[test]
fn error_03_capacity_below_required_bytes_aborts() {
    compare_signal("capacity_below", 6);
}

#[cfg(unix)]
#[test]
fn generic_01_null_output_matches_c() {
    compare_signal("null_output", 11);
}

#[cfg(unix)]
#[test]
fn generic_02_null_input_with_nonzero_length_matches_c() {
    compare_signal("null_input_nonzero", 11);
}

#[test]
fn generic_03_null_input_with_zero_length_succeeds() {
    let libraries = Libraries::load();
    let initial_output = [0xa5, 0x5a, 0xc3, 0x3c, 0x99];
    compare_success(&libraries, std::ptr::null(), 0, 1, &initial_output);
}

#[test]
fn generic_04_zero_length_minimum_capacity_succeeds() {
    let libraries = Libraries::load();
    let input = [];
    let initial_output = [0x11, 0x22, 0x33, 0x44, 0x55];
    compare_success(&libraries, input.as_ptr(), 0, 1, &initial_output);
}

#[cfg(unix)]
#[test]
fn generic_05_near_oversized_boundary_with_zero_capacity_aborts() {
    compare_signal("near_threshold_zero_capacity", 6);
}

#[cfg(unix)]
#[test]
fn generic_06_maximum_length_aborts() {
    compare_signal("maximum_length", 6);
}

#[test]
#[ignore = "invoked in an isolated process by error-path tests"]
fn ffi_child() {
    let library_kind = std::env::var("BIN2HEX_CHILD_LIBRARY").expect("child library kind is set");
    let case = std::env::var("BIN2HEX_CHILD_CASE").expect("child case is set");
    let path = match library_kind.as_str() {
        "c" => c_library_path(),
        "rust" => rust_library_path(),
        other => panic!("unknown child library kind: {other}"),
    };

    // SAFETY: The selected file is one of the two project shared libraries.
    let library = unsafe { Library::new(path).expect("load child shared library") };
    match case.as_str() {
        "oversized_threshold" => {
            // SAFETY: The C guard aborts before either null pointer is dereferenced.
            unsafe {
                Libraries::invoke(
                    &library,
                    std::ptr::null_mut(),
                    usize::MAX,
                    std::ptr::null(),
                    usize::MAX / 2,
                );
            }
        }
        "capacity_equal" => {
            let mut output = [0_u8; 17];
            let input = [0_u8; 8];
            // SAFETY: The guard aborts because 16 is not greater than 8 * 2.
            unsafe {
                Libraries::invoke(
                    &library,
                    output.as_mut_ptr().cast(),
                    16,
                    input.as_ptr(),
                    input.len(),
                );
            }
        }
        "capacity_below" => {
            let mut output = [0_u8; 17];
            let input = [0_u8; 8];
            // SAFETY: The guard aborts because 15 is less than 8 * 2.
            unsafe {
                Libraries::invoke(
                    &library,
                    output.as_mut_ptr().cast(),
                    15,
                    input.as_ptr(),
                    input.len(),
                );
            }
        }
        "null_output" => {
            // SAFETY: Deliberately exercises the C null-output behavior.
            unsafe {
                Libraries::invoke(&library, std::ptr::null_mut(), 1, std::ptr::null(), 0);
            }
        }
        "null_input_nonzero" => {
            let mut output = [0_u8; 3];
            // SAFETY: Deliberately exercises the C null-input behavior.
            unsafe {
                Libraries::invoke(
                    &library,
                    output.as_mut_ptr().cast(),
                    output.len(),
                    std::ptr::null(),
                    1,
                );
            }
        }
        "near_threshold_zero_capacity" => {
            // SAFETY: The capacity guard aborts before either pointer is dereferenced.
            unsafe {
                Libraries::invoke(
                    &library,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                    usize::MAX / 2 - 1,
                );
            }
        }
        "maximum_length" => {
            // SAFETY: The oversized-length guard aborts before pointer dereferences.
            unsafe {
                Libraries::invoke(
                    &library,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                    usize::MAX,
                );
            }
        }
        other => panic!("unknown child case: {other}"),
    }

    std::process::exit(0);
}
