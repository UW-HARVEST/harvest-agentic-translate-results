use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::{File, OpenOptions, remove_file};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type Driver = unsafe extern "C" fn(c_int);

const STDOUT_FILENO: c_int = 1;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn shared_library_paths() -> (PathBuf, PathBuf) {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = crate_dir
        .parent()
        .expect("translation crate must have a parent directory");

    (
        workspace_dir.join("c_src/build/libdriver.so"),
        crate_dir.join("target/release/libdriver.so"),
    )
}

fn unique_capture_file(label: &str) -> (PathBuf, File) {
    loop {
        let sequence = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "driver-differential-{}-{label}-{sequence}.out",
            std::process::id()
        ));

        match OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&path)
        {
            Ok(file) => return (path, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("failed to create capture file {}: {error}", path.display()),
        }
    }
}

fn capture_calls(driver: &Symbol<'_, Driver>, values: &[i32], label: &str) -> Vec<u8> {
    let _stdout_guard = STDOUT_LOCK.lock().expect("stdout mutex poisoned");
    let (path, mut output_file) = unique_capture_file(label);

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "pre-capture fflush failed");

        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "dup(stdout) failed");
        assert_eq!(
            dup2(output_file.as_raw_fd(), STDOUT_FILENO),
            STDOUT_FILENO,
            "redirecting stdout failed"
        );

        for &value in values {
            driver(value);
        }

        assert_eq!(
            fflush(std::ptr::null_mut()),
            0,
            "captured-output fflush failed"
        );
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restoring stdout failed"
        );
        assert_eq!(close(saved_stdout), 0, "closing saved stdout failed");
    }

    output_file
        .seek(SeekFrom::Start(0))
        .expect("failed to rewind capture file");
    let mut output = Vec::new();
    output_file
        .read_to_end(&mut output)
        .expect("failed to read captured output");
    drop(output_file);
    remove_file(&path).expect("failed to remove capture file");
    output
}

fn randomized_inputs() -> Vec<i32> {
    let mut values = vec![
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        i32::MAX - 1,
        i32::MAX,
        0x0102_0304,
        0x7f00_ff80,
        u32::MAX as i32,
    ];

    // Fixed-seed xorshift64*: reproducible full-width C-int bit patterns.
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..4096 {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let random = state.wrapping_mul(0x2545_f491_4f6c_dd1d);
        values.push((random as u32) as i32);
    }

    values
}

fn assert_exists(path: &Path) {
    assert!(
        path.is_file(),
        "required shared library does not exist: {}",
        path.display()
    );
}

#[test]
fn config_1_driver_matches_for_boundaries_and_randomized_inputs() {
    let (c_path, rust_path) = shared_library_paths();
    assert_exists(&c_path);
    assert_exists(&rust_path);

    let inputs = randomized_inputs();

    unsafe {
        let c_library = Library::new(&c_path).expect("failed to load C shared library");
        let rust_library = Library::new(&rust_path).expect("failed to load Rust shared library");
        let c_driver: Symbol<'_, Driver> =
            c_library.get(b"driver\0").expect("C driver export missing");
        let rust_driver: Symbol<'_, Driver> = rust_library
            .get(b"driver\0")
            .expect("Rust driver export missing");

        let c_output = capture_calls(&c_driver, &inputs, "c");
        let rust_output = capture_calls(&rust_driver, &inputs, "rust");

        assert_eq!(
            c_output.len(),
            inputs.len() * 33,
            "C must print 16 bytes as hex plus newline per call"
        );
        assert_eq!(rust_output, c_output, "native stdout differs byte-for-byte");
    }
}
