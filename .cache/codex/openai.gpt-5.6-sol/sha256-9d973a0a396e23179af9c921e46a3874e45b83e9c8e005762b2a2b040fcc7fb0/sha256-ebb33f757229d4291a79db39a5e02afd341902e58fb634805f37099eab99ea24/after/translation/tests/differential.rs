use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_int, c_uint, c_void};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

const ARRAY_SIZE: usize = 256 * 1024;
const RANDOM_ARRAY_CASES: usize = 12;
const WORKER_ENV: &str = "LONG_DIFF_WORKER_SEED";

type Perform = unsafe extern "C" fn();
type LongExec = unsafe extern "C" fn(c_uint);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
}

fn library_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c = env::var_os("LONG_C_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join("../c_src/build/liblong.so"));
    let rust = manifest.join("target/release/liblong.so");
    assert!(c.is_file(), "missing C shared library: {}", c.display());
    assert!(
        rust.is_file(),
        "missing Rust shared library: {}; run cargo build --release first",
        rust.display()
    );
    (c, rust)
}

unsafe fn load_api(
    path: &Path,
) -> (
    Library,
    *mut c_int,
    unsafe extern "C" fn(),
    unsafe extern "C" fn(c_uint),
) {
    let library = unsafe { Library::new(path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
    let array: Symbol<*mut c_int> = unsafe { library.get(b"array") }
        .unwrap_or_else(|error| panic!("failed to load array from {}: {error}", path.display()));
    let perform: Symbol<Perform> = unsafe { library.get(b"perform_expensive_operations") }
        .unwrap_or_else(|error| {
            panic!(
                "failed to load perform_expensive_operations from {}: {error}",
                path.display()
            )
        });
    let long_exec: Symbol<LongExec> =
        unsafe { library.get(b"long_exec") }.unwrap_or_else(|error| {
            panic!("failed to load long_exec from {}: {error}", path.display())
        });
    let array = *array;
    let perform = *perform;
    let long_exec = *long_exec;
    (library, array, perform, long_exec)
}

fn next_random(state: &mut u64) -> u32 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    (x.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
}

unsafe fn write_array(destination: *mut c_int, values: &[c_int]) {
    assert_eq!(values.len(), ARRAY_SIZE);
    unsafe { ptr::copy_nonoverlapping(values.as_ptr(), destination, ARRAY_SIZE) };
}

unsafe fn copy_array(source: *const c_int) -> Vec<c_int> {
    unsafe { std::slice::from_raw_parts(source, ARRAY_SIZE) }.to_vec()
}

fn assert_arrays_equal(context: &str, c_values: &[c_int], rust_values: &[c_int]) {
    assert_eq!(c_values.len(), rust_values.len());
    if let Some(index) = c_values
        .iter()
        .zip(rust_values)
        .position(|(c_value, rust_value)| c_value != rust_value)
    {
        panic!(
            "{context}: first array mismatch at index {index}: C={} ({:#010x}), Rust={} ({:#010x})",
            c_values[index], c_values[index] as u32, rust_values[index], rust_values[index] as u32
        );
    }
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    let mut pipe_fds = [-1; 2];
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0, "pipe failed");
    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0, "dup(stdout) failed");
    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0, "fflush failed");
    assert_eq!(unsafe { dup2(pipe_fds[1], 1) }, 1, "dup2 to pipe failed");
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0, "close pipe writer failed");

    call();

    assert_eq!(unsafe { fflush(ptr::null_mut()) }, 0, "fflush failed");
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1, "restore stdout failed");
    assert_eq!(
        unsafe { close(saved_stdout) },
        0,
        "close saved stdout failed"
    );

    let mut output = Vec::new();
    let mut buffer = [0_u8; 128];
    loop {
        let count = unsafe {
            read(
                pipe_fds[0],
                buffer.as_mut_ptr().cast::<c_void>(),
                buffer.len(),
            )
        };
        assert!(count >= 0, "read captured stdout failed");
        if count == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..count as usize]);
    }
    assert_eq!(unsafe { close(pipe_fds[0]) }, 0, "close pipe reader failed");
    output
}

#[test]
fn differential_perform_expensive_operations_randomized_full_arrays() {
    let (c_path, rust_path) = library_paths();
    let (c_library, c_array, c_perform, _) = unsafe { load_api(&c_path) };
    let (rust_library, rust_array, rust_perform, _) = unsafe { load_api(&rust_path) };
    let mut random_state = 0x4d59_5df4_d0f3_3173_u64;

    for case in 0..RANDOM_ARRAY_CASES {
        let mut input = Vec::with_capacity(ARRAY_SIZE);
        for _ in 0..ARRAY_SIZE {
            input.push(next_random(&mut random_state) as c_int);
        }
        input[0] = 0;
        input[1] = 1;
        input[2] = -1;
        input[3] = c_int::MIN;
        input[4] = c_int::MAX;
        input[5] = case as c_int;
        input[6] = -(case as c_int);

        unsafe {
            write_array(c_array, &input);
            write_array(rust_array, &input);
            c_perform();
            rust_perform();
        }

        let c_output = unsafe { copy_array(c_array) };
        let rust_output = unsafe { copy_array(rust_array) };
        assert_arrays_equal(
            &format!("randomized full-array case {case}"),
            &c_output,
            &rust_output,
        );
    }

    drop(rust_library);
    drop(c_library);
}

fn long_exec_seed_cases() -> Vec<u32> {
    let mut seeds = vec![0, u32::MAX];
    let mut random_state = 0xa076_1d64_78bd_642f_u64;
    for _ in 0..6 {
        seeds.push(next_random(&mut random_state));
    }
    seeds
}

#[test]
fn differential_long_exec_worker() {
    let Ok(seed_text) = env::var(WORKER_ENV) else {
        return;
    };
    let seed: u32 = seed_text.parse().expect("invalid worker seed");
    let (c_path, rust_path) = library_paths();
    let (c_library, c_array, _, c_long_exec) = unsafe { load_api(&c_path) };
    let (rust_library, rust_array, _, rust_long_exec) = unsafe { load_api(&rust_path) };

    let c_stdout = unsafe { capture_stdout(|| c_long_exec(seed)) };
    let c_output = unsafe { copy_array(c_array) };
    let rust_stdout = unsafe { capture_stdout(|| rust_long_exec(seed)) };
    let rust_output = unsafe { copy_array(rust_array) };

    assert_eq!(
        c_stdout,
        rust_stdout,
        "long_exec({seed}) stdout differs: C={:?}, Rust={:?}",
        String::from_utf8_lossy(&c_stdout),
        String::from_utf8_lossy(&rust_stdout)
    );
    assert_arrays_equal(&format!("long_exec({seed})"), &c_output, &rust_output);

    drop(rust_library);
    drop(c_library);
}

#[test]
#[ignore = "the unoptimized C reference exceeds the mandatory 600-second command limit"]
fn differential_long_exec_randomized_and_boundary_seeds() {
    if env::var_os(WORKER_ENV).is_some() {
        return;
    }

    let test_binary = env::current_exe().expect("cannot locate integration-test executable");
    let mut workers = Vec::new();
    for seed in long_exec_seed_cases() {
        let child = Command::new(&test_binary)
            .args([
                "--exact",
                "differential_long_exec_worker",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(WORKER_ENV, seed.to_string())
            .spawn()
            .unwrap_or_else(|error| {
                panic!("failed to spawn long_exec worker for seed {seed}: {error}")
            });
        workers.push((seed, child));
    }

    let mut failures = Vec::new();
    for (seed, child) in workers {
        let output = child
            .wait_with_output()
            .unwrap_or_else(|error| panic!("failed to wait for long_exec worker {seed}: {error}"));
        if !output.status.success() {
            failures.push(format!(
                "seed {seed}: status={}; stdout={}; stderr={}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "long_exec differential workers failed:\n{}",
        failures.join("\n")
    );
}
