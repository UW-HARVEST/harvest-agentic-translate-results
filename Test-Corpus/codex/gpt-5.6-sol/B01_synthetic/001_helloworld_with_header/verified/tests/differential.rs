use libloading::{Library, Symbol};
use std::collections::BTreeSet;
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::Mutex;

type EntryPoint = unsafe extern "C" fn() -> c_int;

const STDOUT_FILENO: c_int = 1;
const EXPECTED_LINE: &[u8] = b"Hello World!\n";
const RANDOMIZED_BATCHES: usize = 64;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver_c.so")
}

fn rust_library_path() -> PathBuf {
    let executable = std::env::current_exe().expect("get integration test path");
    let profile_dir = executable
        .parent()
        .and_then(Path::parent)
        .expect("integration test must run from target/<profile>/deps");
    profile_dir.join(format!(
        "{}driver{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    ))
}

fn capture_calls(function: EntryPoint, count: usize) -> (Vec<c_int>, Vec<u8>) {
    let _stdout_guard = STDOUT_LOCK.lock().expect("lock stdout capture");

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "flush stdout before capture");

        let mut pipe_fds = [-1; 2];
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "create stdout pipe");

        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "redirect stdout"
        );
        assert_eq!(close(pipe_fds[1]), 0, "close extra pipe writer");

        let results = (0..count).map(|_| function()).collect();
        let flush_result = fflush(ptr::null_mut());
        let restore_result = dup2(saved_stdout, STDOUT_FILENO);
        let close_result = close(saved_stdout);

        assert_eq!(flush_result, 0, "flush captured stdout");
        assert_eq!(restore_result, STDOUT_FILENO, "restore stdout");
        assert_eq!(close_result, 0, "close saved stdout");

        let mut output = Vec::new();
        File::from_raw_fd(pipe_fds[0])
            .read_to_end(&mut output)
            .expect("read captured stdout");
        (results, output)
    }
}

fn compare_entry_point(symbol_name: &[u8], seed: u64) {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert!(c_path.is_file(), "missing C shared library: {c_path:?}");
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {rust_path:?}"
    );

    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared library");
        let rust_library = Library::new(&rust_path).expect("load Rust shared library");
        let c_function: Symbol<EntryPoint> = c_library.get(symbol_name).expect("resolve C symbol");
        let rust_function: Symbol<EntryPoint> =
            rust_library.get(symbol_name).expect("resolve Rust symbol");

        let mut state = seed;
        for batch in 0..RANDOMIZED_BATCHES {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let call_count = ((state >> 32) as usize % 8) + 1;

            let (c_results, c_output) = capture_calls(*c_function, call_count);
            let (rust_results, rust_output) = capture_calls(*rust_function, call_count);
            let expected_output = EXPECTED_LINE.repeat(call_count);

            assert_eq!(rust_results, c_results, "return mismatch in batch {batch}");
            assert_eq!(c_results, vec![0; call_count], "unexpected C return");
            assert_eq!(rust_output, c_output, "byte mismatch in batch {batch}");
            assert_eq!(c_output, expected_output, "unexpected C output");
        }
    }
}

fn dynamic_definitions(path: &Path) -> BTreeSet<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        output.status.success(),
        "nm failed for {path:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .expect("nm output must be UTF-8")
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_owned)
        .collect()
}

#[test]
fn complete_surface_matches_through_ffi() {
    compare_entry_point(b"helloworld\0", 0x6865_6c6c_6f77_6f72);
    compare_entry_point(b"main\0", 0x6d61_696e_5f66_6669);

    let c_symbols = dynamic_definitions(&c_library_path());
    let rust_symbols = dynamic_definitions(&rust_library_path());
    let missing: Vec<_> = c_symbols.difference(&rust_symbols).collect();
    assert!(missing.is_empty(), "Rust is missing C symbols: {missing:?}");
}
