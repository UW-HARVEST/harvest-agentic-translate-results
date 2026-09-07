use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type Driver = unsafe extern "C" fn(c_int);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct StdoutRestore {
    saved_fd: c_int,
}

impl Drop for StdoutRestore {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            assert_eq!(dup2(self.saved_fd, 1), 1, "failed to restore stdout");
            assert_eq!(close(self.saved_fd), 0, "failed to close saved stdout");
        }
    }
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _lock = STDOUT_LOCK.lock().expect("stdout capture mutex poisoned");
    let capture_path = unique_capture_path();
    let capture_file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&capture_path)
        .expect("failed to create stdout capture file");

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "failed to flush stdout");
        let saved_fd = dup(1);
        assert!(saved_fd >= 0, "failed to duplicate stdout");
        let restore = StdoutRestore { saved_fd };
        assert_eq!(
            dup2(capture_file.as_raw_fd(), 1),
            1,
            "failed to redirect stdout"
        );

        call();
        assert_eq!(fflush(std::ptr::null_mut()), 0, "failed to flush capture");
        drop(restore);
    }

    drop(capture_file);
    let bytes = fs::read(&capture_path).expect("failed to read stdout capture");
    fs::remove_file(&capture_path).expect("failed to remove stdout capture");
    bytes
}

fn unique_capture_path() -> PathBuf {
    let id = CAPTURE_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver-differential-{}-{id}.stdout",
        std::process::id()
    ))
}

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    (
        crate_root.join("../c_src/build/libdriver.so"),
        crate_root.join("target/release/libdriver.so"),
    )
}

fn compare_one(c_driver: &Symbol<'_, Driver>, rust_driver: &Symbol<'_, Driver>, x: c_int) {
    let c_output = capture_stdout(|| unsafe { c_driver(x) });
    let rust_output = capture_stdout(|| unsafe { rust_driver(x) });
    assert_eq!(rust_output, c_output, "stdout differed for x={x}");
}

fn next_random(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 32) as u32
}

#[test]
fn all_configuration_rows_match_through_shared_library_exports() {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C shared library: {c_path:?}");
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {rust_path:?}"
    );

    unsafe {
        let c_library = Library::new(&c_path).expect("failed to load C shared library");
        let rust_library = Library::new(&rust_path).expect("failed to load Rust shared library");
        let c_driver: Symbol<'_, Driver> =
            c_library.get(b"driver\0").expect("C driver symbol missing");
        let rust_driver: Symbol<'_, Driver> = rust_library
            .get(b"driver\0")
            .expect("Rust driver symbol missing");

        // CONFIGS.md row 1: negative values produce no records.
        compare_one(&c_driver, &rust_driver, c_int::MIN);
        let mut seed = 0x5eed_c0de_d15c_a11u64;
        for _ in 0..128 {
            let x = -((next_random(&mut seed) % 10_000) as c_int + 1);
            compare_one(&c_driver, &rust_driver, x);
        }

        // CONFIGS.md row 2: the zero boundary produces no records.
        for _ in 0..32 {
            compare_one(&c_driver, &rust_driver, 0);
        }

        // CONFIGS.md row 3: one iteration produces exactly one record.
        for _ in 0..32 {
            compare_one(&c_driver, &rust_driver, 1);
        }

        // CONFIGS.md row 4: randomized positive multi-record inputs.
        for _ in 0..128 {
            let x = (next_random(&mut seed) % 255 + 2) as c_int;
            compare_one(&c_driver, &rust_driver, x);
        }
    }
}
