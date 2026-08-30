use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int);

const STDOUT_FILENO: c_int = 1;
const RANDOM_CASES: usize = 8_192;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn shared_object_paths() -> (PathBuf, PathBuf) {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = crate_root.join("../c_src/build/libdriver.so");
    let rust_library = crate_root.join("target/release/libdriver.so");

    assert!(
        c_library.is_file(),
        "C shared library is missing: {}",
        c_library.display()
    );
    assert!(
        rust_library.is_file(),
        "Rust shared library is missing: {}; run `cargo build --release` first",
        rust_library.display()
    );

    (c_library, rust_library)
}

fn test_values() -> Vec<c_int> {
    let mut values = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -151,
        -150,
        -149,
        -1,
        0,
        1,
        (c_int::MAX - 300) / 2,
        (c_int::MAX - 300) / 2 + 1,
        c_int::MAX - 1,
        c_int::MAX,
    ];

    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..RANDOM_CASES {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        values.push(state as u32 as c_int);
    }

    values
}

fn capture_calls(driver: &Symbol<'_, Driver>, values: &[c_int], path: &Path) -> Vec<u8> {
    let mut capture = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(path)
        .expect("create stdout capture file");

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush stdout");
    }
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0, "duplicate stdout");
    assert_eq!(
        unsafe { dup2(capture.as_raw_fd(), STDOUT_FILENO) },
        STDOUT_FILENO,
        "redirect stdout"
    );

    for &value in values {
        unsafe { driver(value) };
    }

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "close saved stdout");
    }

    capture.seek(SeekFrom::Start(0)).expect("rewind capture");
    let mut output = Vec::new();
    capture.read_to_end(&mut output).expect("read capture");
    output
}

fn load_driver(library: &Library) -> Symbol<'_, Driver> {
    unsafe { library.get(b"driver\0") }.expect("load exported driver symbol")
}

#[test]
fn driver_matches_for_boundaries_and_fixed_seed_random_inputs() {
    let _stdout_guard = STDOUT_LOCK.lock().expect("lock stdout capture");
    let (c_path, rust_path) = shared_object_paths();
    let temp_dir = std::env::temp_dir();
    let process_id = std::process::id();
    let c_capture_path = temp_dir.join(format!("driver-c-{process_id}.out"));
    let rust_capture_path = temp_dir.join(format!("driver-rust-{process_id}.out"));
    let values = test_values();

    let c_library = unsafe { Library::new(&c_path) }.expect("load C shared library");
    let rust_library = unsafe { Library::new(&rust_path) }.expect("load Rust shared library");
    let c_output = capture_calls(&load_driver(&c_library), &values, &c_capture_path);
    let rust_output = capture_calls(&load_driver(&rust_library), &values, &rust_capture_path);

    let _ = File::open(&c_capture_path).and_then(|_| std::fs::remove_file(&c_capture_path));
    let _ = File::open(&rust_capture_path).and_then(|_| std::fs::remove_file(&rust_capture_path));

    assert_eq!(
        c_output.split(|&byte| byte == b'\n').count() - 1,
        values.len(),
        "C library must emit one newline-terminated result per call"
    );
    assert_eq!(rust_output, c_output);
}
