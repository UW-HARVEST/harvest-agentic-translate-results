use libloading::{Library, Symbol};
use std::env;
use std::ffi::{c_int, c_uint, c_void};
use std::fs;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

const ARRAY_SIZE: usize = 256 * 1024;
const WORKER_LIB: &str = "LONG_DIFF_WORKER_LIB";
const WORKER_OUT: &str = "LONG_DIFF_WORKER_OUT";
const WORKER_SEED: &str = "LONG_DIFF_WORKER_SEED";

type Operation = unsafe extern "C" fn();
type LongExec = unsafe extern "C" fn(c_uint);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation crate must have a parent")
        .to_path_buf()
}

fn c_library() -> PathBuf {
    project_root().join("c_src/build/liblong.so")
}

fn rust_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/liblong.so")
}

unsafe fn array_symbol(library: &Library) -> *mut c_int {
    let symbol: Symbol<*mut c_int> = unsafe {
        library
            .get(b"array\0")
            .expect("shared library must export array")
    };
    *symbol
}

unsafe fn operation_symbol(library: &Library) -> Operation {
    let symbol: Symbol<Operation> = unsafe {
        library
            .get(b"perform_expensive_operations\0")
            .expect("shared library must export perform_expensive_operations")
    };
    *symbol
}

unsafe fn long_exec_symbol(library: &Library) -> LongExec {
    let symbol: Symbol<LongExec> = unsafe {
        library
            .get(b"long_exec\0")
            .expect("shared library must export long_exec")
    };
    *symbol
}

fn load(path: &Path) -> Library {
    assert!(
        path.is_file(),
        "shared library is missing: {}",
        path.display()
    );
    unsafe { Library::new(path).unwrap_or_else(|error| panic!("load {}: {error}", path.display())) }
}

#[test]
fn exported_surface_is_loadable() {
    for path in [c_library(), rust_library()] {
        let library = load(&path);
        unsafe {
            let _ = array_symbol(&library);
            let _ = operation_symbol(&library);
            let _ = long_exec_symbol(&library);
        }
    }
}

#[test]
fn configuration_1_low_level_operation_matches() {
    let c = load(&c_library());
    let rust = load(&rust_library());

    unsafe {
        let c_array = array_symbol(&c);
        let rust_array = array_symbol(&rust);
        let mut state = 0x4d59_5df4_d0f3_3173_u64;

        for index in 0..ARRAY_SIZE {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let value = state as c_int;
            c_array.add(index).write(value);
            rust_array.add(index).write(value);
        }

        let boundaries = [c_int::MIN, -2, -1, 0, 1, 2, c_int::MAX];
        for (index, value) in boundaries.into_iter().enumerate() {
            c_array.add(index).write(value);
            rust_array.add(index).write(value);
        }

        operation_symbol(&c)();
        operation_symbol(&rust)();

        let c_result = std::slice::from_raw_parts(c_array, ARRAY_SIZE);
        let rust_result = std::slice::from_raw_parts(rust_array, ARRAY_SIZE);
        assert_eq!(c_result, rust_result);
    }
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let mut pipe_fds = [-1; 2];
    assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);

    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], 1) }, 1);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    call();

    assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1);
    assert_eq!(unsafe { close(saved_stdout) }, 0);

    let mut output = Vec::new();
    let mut reader = unsafe { fs::File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_end(&mut output)
        .expect("read captured stdout");
    output
}

fn write_worker_result(library_path: &Path, output_path: &Path, seed: c_uint) {
    let library = load(library_path);
    unsafe {
        let array = array_symbol(&library);
        let long_exec = long_exec_symbol(&library);
        let stdout = capture_stdout(|| long_exec(seed));
        let array_bytes =
            std::slice::from_raw_parts(array.cast::<u8>(), ARRAY_SIZE * size_of::<c_int>());

        let mut result = Vec::with_capacity(4 + stdout.len() + array_bytes.len());
        result.extend_from_slice(&(stdout.len() as u32).to_le_bytes());
        result.extend_from_slice(&stdout);
        result.extend_from_slice(array_bytes);
        fs::write(output_path, result).expect("write worker result");
    }
}

#[test]
fn long_exec_worker() {
    let Ok(library_path) = env::var(WORKER_LIB) else {
        return;
    };
    let output_path = env::var(WORKER_OUT).expect("worker output path");
    let seed = env::var(WORKER_SEED)
        .expect("worker seed")
        .parse::<c_uint>()
        .expect("worker seed must be an unsigned int");
    write_worker_result(Path::new(&library_path), Path::new(&output_path), seed);
}

struct Worker {
    child: Child,
    seed: c_uint,
    implementation: &'static str,
}

fn spawn_worker(
    test_binary: &Path,
    library_path: &Path,
    output_path: PathBuf,
    seed: c_uint,
    implementation: &'static str,
) -> Worker {
    let child = Command::new(test_binary)
        .arg("--exact")
        .arg("long_exec_worker")
        .arg("--nocapture")
        .env(WORKER_LIB, library_path)
        .env(WORKER_OUT, &output_path)
        .env(WORKER_SEED, seed.to_string())
        .env("RUST_TEST_THREADS", "1")
        .spawn()
        .expect("spawn long_exec worker");
    Worker {
        child,
        seed,
        implementation,
    }
}

#[test]
fn configuration_2_end_to_end_matches() {
    if env::var_os(WORKER_LIB).is_some() {
        return;
    }

    let test_binary = env::current_exe().expect("current test executable");
    let temp_prefix = env::temp_dir().join(format!("long-differential-{}", std::process::id()));
    let seeds = [
        0,
        c_uint::MAX,
        0x243f_6a88,
        0x85a3_08d3,
        0x1319_8a2e,
        0x0370_7344,
    ];

    let mut workers = Vec::with_capacity(seeds.len() * 2);
    for (index, seed) in seeds.into_iter().enumerate() {
        workers.push(spawn_worker(
            &test_binary,
            &c_library(),
            temp_prefix.with_extension(format!("{index}.c")),
            seed,
            "C",
        ));
        workers.push(spawn_worker(
            &test_binary,
            &rust_library(),
            temp_prefix.with_extension(format!("{index}.rust")),
            seed,
            "Rust",
        ));
    }

    for worker in &mut workers {
        let status = worker.child.wait().expect("wait for long_exec worker");
        assert!(
            status.success(),
            "{} long_exec worker failed for seed {}",
            worker.implementation,
            worker.seed
        );
    }

    for index in 0..seeds.len() {
        let c_path = temp_prefix.with_extension(format!("{index}.c"));
        let rust_path = temp_prefix.with_extension(format!("{index}.rust"));
        let c_result = fs::read(&c_path).expect("read C worker result");
        let rust_result = fs::read(&rust_path).expect("read Rust worker result");
        assert_eq!(
            c_result, rust_result,
            "long_exec result differs for seed {}",
            seeds[index]
        );
        fs::remove_file(c_path).expect("remove C worker result");
        fs::remove_file(rust_path).expect("remove Rust worker result");
    }
}
