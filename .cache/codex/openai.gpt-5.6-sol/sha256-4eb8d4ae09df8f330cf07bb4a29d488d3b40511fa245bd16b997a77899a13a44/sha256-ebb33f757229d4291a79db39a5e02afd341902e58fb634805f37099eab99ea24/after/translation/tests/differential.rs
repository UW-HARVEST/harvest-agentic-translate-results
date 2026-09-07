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

type Driver = unsafe extern "C" fn(c_int);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipe_fds: *mut c_int) -> c_int;
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    manifest_dir().join("target/release/libdriver.so")
}

fn assert_library_exists(path: &Path) {
    assert!(
        path.is_file(),
        "missing shared library {}; build both release libraries before testing",
        path.display()
    );
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    const STDOUT_FILENO: c_int = 1;

    let _guard = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "pre-capture fflush failed");
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "pipe failed");

        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "dup(stdout) failed");
        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "redirecting stdout failed"
        );

        call();

        assert_eq!(fflush(ptr::null_mut()), 0, "captured fflush failed");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restoring stdout failed"
        );
        assert_eq!(close(saved_stdout), 0, "closing saved stdout failed");
        assert_eq!(close(pipe_fds[1]), 0, "closing pipe writer failed");

        let mut output = Vec::new();
        let mut reader = File::from_raw_fd(pipe_fds[0]);
        reader
            .read_to_end(&mut output)
            .expect("reading captured stdout failed");
        output
    }
}

fn edge_inputs() -> Vec<c_int> {
    vec![
        c_int::MIN,
        c_int::MIN + 1,
        -1_073_741_975,
        -1_073_741_974,
        -1_073_741_824,
        -151,
        -150,
        -149,
        -1,
        0,
        1,
        149,
        150,
        151,
        1_073_741_673,
        1_073_741_674,
        1_073_741_823,
        1_073_741_824,
        c_int::MAX - 1,
        c_int::MAX,
    ]
}

fn randomized_inputs(count: usize) -> Vec<c_int> {
    // Fixed-seed SplitMix64: reproducible and spans all 32 input bits.
    let mut state = 0x6a09_e667_f3bc_c909_u64;
    (0..count)
        .map(|_| {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            (z ^ (z >> 31)) as u32 as c_int
        })
        .collect()
}

fn defined_dynamic_symbols(path: &Path) -> BTreeSet<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("failed to execute nm");
    assert!(
        output.status.success(),
        "nm failed for {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .expect("nm output was not UTF-8")
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_owned)
        .collect()
}

#[test]
fn all_c_defined_dynamic_symbols_are_exported_by_rust() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert_library_exists(&c_path);
    assert_library_exists(&rust_path);

    let c_symbols = defined_dynamic_symbols(&c_path);
    let rust_symbols = defined_dynamic_symbols(&rust_path);
    let missing: Vec<_> = c_symbols.difference(&rust_symbols).cloned().collect();

    assert_eq!(c_symbols, BTreeSet::from(["driver".to_owned()]));
    assert!(missing.is_empty(), "symbols missing from Rust: {missing:?}");
}

#[test]
fn driver_matches_for_full_domain_boundaries_and_randomized_inputs() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    assert_library_exists(&c_path);
    assert_library_exists(&rust_path);

    unsafe {
        let c_library = Library::new(&c_path).expect("failed to load C library");
        let rust_library = Library::new(&rust_path).expect("failed to load Rust library");
        let c_driver: Symbol<'_, Driver> =
            c_library.get(b"driver\0").expect("C driver symbol missing");
        let rust_driver: Symbol<'_, Driver> = rust_library
            .get(b"driver\0")
            .expect("Rust driver symbol missing");

        let mut inputs = edge_inputs();
        inputs.extend(randomized_inputs(8_192));

        for x in inputs {
            let c_output = capture_stdout(|| c_driver(x));
            let rust_output = capture_stdout(|| rust_driver(x));
            assert_eq!(
                rust_output,
                c_output,
                "stdout differed for x={x}: C={:?}, Rust={:?}",
                String::from_utf8_lossy(&c_output),
                String::from_utf8_lossy(&rust_output)
            );
        }
    }
}
