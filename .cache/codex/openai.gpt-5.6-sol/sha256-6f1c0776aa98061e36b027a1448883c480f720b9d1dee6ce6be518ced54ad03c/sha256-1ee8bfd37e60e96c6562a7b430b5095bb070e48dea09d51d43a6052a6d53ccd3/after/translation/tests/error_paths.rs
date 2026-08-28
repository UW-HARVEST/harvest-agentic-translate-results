mod common;

use common::{
    AllocationHarness, Api, Rng, assert_i32_bytes, c_library_path, load_both, rust_library_path,
};
use std::ffi::c_int;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

const SAMPLES: usize = 96;

#[test]
fn error_rows_1_and_2_shift_range_rejections() {
    let (c, rust) = unsafe { load_both() };
    let mut rng = Rng::new(0x4d52_15e9_864b_bf03);

    for row in 1..=2 {
        for sample in 0..SAMPLES {
            let size = rng.i32_between(-16, 64);
            let positions = if row == 1 {
                rng.i32_between(-128, 0)
            } else {
                rng.i32_between(size.max(1), size.max(1) + 128)
            };
            let storage_length = size.max(0) as usize;
            let original: Vec<c_int> = (0..storage_length)
                .map(|_| rng.i32_between(-100_000, 100_000))
                .collect();
            let mut c_values = original.clone();
            let mut rust_values = original.clone();

            let c_pointer = if sample % 3 == 0 {
                std::ptr::null_mut()
            } else {
                c_values.as_mut_ptr()
            };
            let rust_pointer = if sample % 3 == 0 {
                std::ptr::null_mut()
            } else {
                rust_values.as_mut_ptr()
            };
            unsafe {
                (c.shift_array)(c_pointer, size, positions);
                (rust.shift_array)(rust_pointer, size, positions);
            }
            assert_eq!(
                c_values, original,
                "C changed rejected row {row}, sample {sample}"
            );
            assert_eq!(
                rust_values, original,
                "Rust changed rejected row {row}, sample {sample}"
            );
            assert_eq!(c_values, rust_values, "row {row}, sample {sample}");
        }
    }
}

#[test]
fn error_row_4_arity_unsigned_length_rejection() {
    let (c, rust) = unsafe { load_both() };
    let mut rng = Rng::new(0x137b_2480_aec1_69d5);

    for sample in 0..SAMPLES {
        let low_byte = (sample % 2) as i32;
        let turns = rng.i32_between(-100_000, 100_000);
        let len = low_byte + 256 * turns;
        let mut params = [
            rng.i32_between(-1_000, 1_000),
            rng.i32_between(-1_000, 1_000),
            rng.i32_between(-1_000, 1_000),
            rng.i32_between(-1_000, 1_000),
        ];
        let pointer = if sample % 2 == 0 {
            std::ptr::null_mut()
        } else {
            params.as_mut_ptr()
        };
        let c_value = unsafe { (c.arity)(len, pointer) };
        let rust_value = unsafe { (rust.arity)(len, pointer) };
        assert_i32_bytes(4, sample, c_value, rust_value);
        assert_eq!(c_value, -1, "row 4, sample {sample}");
    }
}

fn compile_malloc_shim() -> &'static Path {
    static SHIM: OnceLock<PathBuf> = OnceLock::new();
    SHIM.get_or_init(|| {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let output_directory = manifest.join("target/test-support");
        std::fs::create_dir_all(&output_directory).unwrap();
        let output = output_directory.join("libmalloc_fail_shim.so");
        let status = Command::new("cc")
            .args(["-shared", "-fPIC", "-O2"])
            .arg(manifest.join("tests/malloc_fail_shim.c"))
            .args(["-ldl", "-o"])
            .arg(&output)
            .status()
            .expect("failed to invoke cc for malloc failure shim");
        assert!(status.success(), "failed to compile malloc failure shim");
        output
    })
}

fn worker_output(test_name: &str, library: &Path, extra_environment: &[(&str, &str)]) -> Output {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", test_name, "--nocapture"])
        .env("FFI_WORKER_LIBRARY", library);
    for (key, value) in extra_environment {
        command.env(key, value);
    }
    command.output().expect("failed to run isolated FFI worker")
}

#[test]
fn malloc_failure_worker() {
    let Some(library) = std::env::var_os("FFI_MALLOC_FAILURE_WORKER") else {
        return;
    };
    let api = unsafe { Api::load(Path::new(&library)) };
    let result = unsafe { (api.compare_allocations)(17, 29) };
    assert_eq!(result, -1);
}

#[test]
fn error_row_3_allocation_failure() {
    let shim = compile_malloc_shim();
    for failure in ["first", "second", "both"] {
        for library in [c_library_path(), rust_library_path()] {
            let output = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "malloc_failure_worker", "--nocapture"])
                .env("LD_PRELOAD", shim)
                .env("FAIL_COMPARE_ALLOCATIONS_MALLOC", failure)
                .env("FFI_MALLOC_FAILURE_WORKER", &library)
                .output()
                .expect("failed to run allocation-failure worker");
            assert!(
                output.status.success(),
                "{failure} failure in {} worker failed:\nstdout:\n{}\nstderr:\n{}",
                library.display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

#[test]
fn generic_noncrashing_boundaries() {
    let (c, rust) = unsafe { load_both() };

    unsafe {
        (c.shift_array)(std::ptr::null_mut(), 0, 0);
        (rust.shift_array)(std::ptr::null_mut(), 0, 0);
    }
    assert_i32_bytes(
        4,
        0,
        unsafe { (c.arity)(0, std::ptr::null_mut()) },
        unsafe { (rust.arity)(0, std::ptr::null_mut()) },
    );
    assert_i32_bytes(
        12,
        0,
        unsafe { (c.apply_bitmask)(0x1234_5678, 4) },
        unsafe { (rust.apply_bitmask)(0x1234_5678, 4) },
    );

    let mut params = [4, -17, 3, 9];
    let pointer = params.as_mut_ptr();
    let mut allocations = AllocationHarness::new();
    allocations.compare(
        53,
        0,
        || unsafe { (c.arity)(i32::MAX, pointer) },
        || unsafe { (rust.arity)(i32::MAX, pointer) },
    );
}

#[test]
fn crash_worker() {
    let Some(case) = std::env::var_os("FFI_CRASH_WORKER") else {
        return;
    };
    let library = std::env::var_os("FFI_WORKER_LIBRARY").unwrap();
    let api = unsafe { Api::load(Path::new(&library)) };

    unsafe {
        match case.to_str().unwrap() {
            "shift-null" => (api.shift_array)(std::ptr::null_mut(), 2, 1),
            "shift-null-oversized" => (api.shift_array)(std::ptr::null_mut(), i32::MAX, 1),
            "process-null" => {
                let _ = (api.process_string)(std::ptr::null());
            }
            "init-null" => (api.init_matrix)(std::ptr::null_mut()),
            "arity-null" => {
                let _ = (api.arity)(2, std::ptr::null_mut());
            }
            unknown => panic!("unknown crash case {unknown}"),
        }
    }
    panic!("invalid pointer call unexpectedly returned");
}

#[test]
fn generic_invalid_pointer_behavior_matches() {
    for case in [
        "shift-null",
        "shift-null-oversized",
        "process-null",
        "init-null",
        "arity-null",
    ] {
        let c_output = worker_output(
            "crash_worker",
            &c_library_path(),
            &[("FFI_CRASH_WORKER", case)],
        );
        let rust_output = worker_output(
            "crash_worker",
            &rust_library_path(),
            &[("FFI_CRASH_WORKER", case)],
        );
        assert!(
            !c_output.status.success() && !rust_output.status.success(),
            "{case}: an invalid pointer call unexpectedly succeeded"
        );
        assert_eq!(
            c_output.status.signal(),
            rust_output.status.signal(),
            "{case}: C status {:?}, Rust status {:?}\nC stderr:\n{}\nRust stderr:\n{}",
            c_output.status,
            rust_output.status,
            String::from_utf8_lossy(&c_output.stderr),
            String::from_utf8_lossy(&rust_output.stderr)
        );
        assert!(
            c_output.status.signal().is_some(),
            "{case}: workers failed without a terminating signal"
        );
    }
}
