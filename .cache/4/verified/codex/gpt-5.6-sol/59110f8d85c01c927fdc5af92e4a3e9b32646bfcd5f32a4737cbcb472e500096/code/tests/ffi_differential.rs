use libloading::Library;
use std::ffi::c_void;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

const ARRAY_LEN: usize = 256 * 1024;
const RANDOM_ARRAY_CASES: usize = 12;
const STDOUT_FILENO: i32 = 1;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: i32) -> i32;
    fn dup(fd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn fflush(stream: *mut c_void) -> i32;
    fn pipe(pipefd: *mut i32) -> i32;
    fn read(fd: i32, buffer: *mut c_void, count: usize) -> isize;
}

struct Api {
    _library: Library,
    array: *mut i32,
    perform_expensive_operations: unsafe extern "C" fn(),
    long_exec: unsafe extern "C" fn(u32),
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let library = unsafe { Library::new(path) }
            .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
        let array = unsafe { *library.get::<*mut i32>(b"array\0").unwrap() };
        let perform_expensive_operations = unsafe {
            *library
                .get::<unsafe extern "C" fn()>(b"perform_expensive_operations\0")
                .unwrap()
        };
        let long_exec = unsafe {
            *library
                .get::<unsafe extern "C" fn(u32)>(b"long_exec\0")
                .unwrap()
        };

        Self {
            _library: library,
            array,
            perform_expensive_operations,
            long_exec,
        }
    }

    unsafe fn write_array(&self, values: &[i32]) {
        assert_eq!(values.len(), ARRAY_LEN);
        unsafe { std::ptr::copy_nonoverlapping(values.as_ptr(), self.array, ARRAY_LEN) };
    }

    unsafe fn array_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.array.cast::<u8>(),
                ARRAY_LEN * std::mem::size_of::<i32>(),
            )
        }
    }
}

fn c_library_path() -> PathBuf {
    std::env::var_os("C_LONG_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("c_src")
                .join("build")
                .join("liblong.so")
        })
}

fn optimized_c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build-release")
        .join("liblong.so")
}

fn rust_library_path() -> PathBuf {
    std::env::var_os("RUST_LONG_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("release")
                .join("liblong.so")
        })
}

fn next_random(state: &mut u64) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 16) as u32
}

fn assert_bytes_equal(context: &str, c_bytes: &[u8], rust_bytes: &[u8]) {
    assert_eq!(
        c_bytes.len(),
        rust_bytes.len(),
        "{context}: output lengths differ"
    );
    if let Some(index) = c_bytes
        .iter()
        .zip(rust_bytes)
        .position(|(c_byte, rust_byte)| c_byte != rust_byte)
    {
        panic!(
            "{context}: byte {index} differs: C={:#04x}, Rust={:#04x}",
            c_bytes[index], rust_bytes[index]
        );
    }
}

fn final_decimal_line(output: &[u8]) -> &[u8] {
    assert_eq!(
        output.last(),
        Some(&b'\n'),
        "missing final newline: {output:?}"
    );
    let mut start = output.len() - 1;
    while start > 0 && output[start - 1].is_ascii_digit() {
        start -= 1;
    }
    assert!(
        start < output.len() - 1,
        "missing decimal digits: {output:?}"
    );
    if start > 0 && output[start - 1] == b'-' {
        start -= 1;
    }
    &output[start..]
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap();
    let mut pipe_fds = [-1; 2];

    std::io::stdout().flush().unwrap();
    assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    call();

    assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, STDOUT_FILENO) }, STDOUT_FILENO);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

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
        assert!(count >= 0);
        if count == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..count as usize]);
    }
    assert_eq!(unsafe { close(pipe_fds[0]) }, 0);
    output
}

#[test]
fn stdout_capture_excludes_harness_output() {
    let output = unsafe { capture_stdout(|| {}) };
    assert!(output.is_empty(), "captured unexpected bytes: {output:?}");
    assert_eq!(final_decimal_line(b"test status ... -123\n"), b"-123\n");
    assert_eq!(final_decimal_line(b"456\n"), b"456\n");
}

#[test]
fn low_level_operation_matches_for_randomized_arrays() {
    assert_eq!(std::mem::size_of::<i32>(), 4);

    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing {}", c_path.display());
    assert!(rust_path.is_file(), "missing {}", rust_path.display());

    let c = unsafe { Api::load(&c_path) };
    let rust = unsafe { Api::load(&rust_path) };
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    let mut input = vec![0_i32; ARRAY_LEN];
    let boundaries = [
        i32::MIN,
        i32::MIN + 1,
        -7,
        -1,
        0,
        1,
        7,
        i32::MAX - 1,
        i32::MAX,
    ];

    for case in 0..RANDOM_ARRAY_CASES {
        for value in &mut input {
            *value = next_random(&mut state) as i32;
        }
        input[..boundaries.len()].copy_from_slice(&boundaries);
        input.rotate_left(case);

        unsafe {
            c.write_array(&input);
            rust.write_array(&input);
            (c.perform_expensive_operations)();
            (rust.perform_expensive_operations)();
            assert_bytes_equal(
                &format!("randomized array case {case}"),
                c.array_bytes(),
                rust.array_bytes(),
            );
        }
    }
}

#[test]
fn long_exec_worker() {
    let Some(seed) = std::env::var_os("DIFF_LONG_SEED") else {
        return;
    };
    let seed = seed
        .to_str()
        .expect("DIFF_LONG_SEED must be UTF-8")
        .parse::<u32>()
        .expect("DIFF_LONG_SEED must be a u32");
    let c = unsafe { Api::load(&c_library_path()) };
    let rust = unsafe { Api::load(&rust_library_path()) };

    let c_stdout = unsafe { capture_stdout(|| (c.long_exec)(seed)) };
    let c_array = unsafe { c.array_bytes().to_vec() };
    let rust_stdout = unsafe { capture_stdout(|| (rust.long_exec)(seed)) };

    assert_bytes_equal(&format!("long_exec({seed}) array"), &c_array, unsafe {
        rust.array_bytes()
    });
    assert_bytes_equal(
        &format!("long_exec({seed}) stdout"),
        final_decimal_line(&c_stdout),
        final_decimal_line(&rust_stdout),
    );
}

#[test]
fn long_exec_matches_for_randomized_seeds() {
    let optimized_c = optimized_c_library_path();
    assert!(
        optimized_c.is_file(),
        "missing optimized C library {}",
        optimized_c.display()
    );

    let mut state = 0xa076_1d64_78bd_642f_u64;
    let mut seeds = vec![0, 1, u32::MAX - 1, u32::MAX];
    for _ in 0..4 {
        seeds.push(next_random(&mut state));
    }

    let executable = std::env::current_exe().unwrap();
    let mut workers = Vec::with_capacity(seeds.len());
    for seed in seeds {
        let child = Command::new(&executable)
            .args(["--exact", "long_exec_worker"])
            .env("C_LONG_SO", &optimized_c)
            .env("DIFF_LONG_SEED", seed.to_string())
            .spawn()
            .unwrap_or_else(|error| panic!("failed to spawn seed {seed} worker: {error}"));
        workers.push((seed, child));
    }

    for (seed, child) in workers {
        let output = child
            .wait_with_output()
            .unwrap_or_else(|error| panic!("failed to wait for seed {seed} worker: {error}"));
        assert!(
            output.status.success(),
            "seed {seed} worker failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
