use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type Driver = unsafe extern "C" fn(c_int);

const STDOUT_FD: c_int = 1;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct StdoutRedirect {
    saved_stdout: c_int,
}

impl Drop for StdoutRedirect {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            assert_eq!(dup2(self.saved_stdout, STDOUT_FD), STDOUT_FD);
            assert_eq!(close(self.saved_stdout), 0);
        }
    }
}

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .expect("translation crate must have a parent directory");

    (
        workspace_root.join("c_src/build/libdriver.so"),
        crate_root.join("target/release/libdriver.so"),
    )
}

fn assert_library_exists(path: &Path) {
    assert!(
        path.is_file(),
        "shared library is missing: {}",
        path.display()
    );
}

fn capture_driver_output(driver: Driver, values: &[c_int]) -> Vec<u8> {
    let _stdout_lock = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let capture_path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{sequence}.stdout",
        std::process::id()
    ));
    let capture_file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&capture_path)
        .expect("create stdout capture file");

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
    }
    let saved_stdout = unsafe { dup(STDOUT_FD) };
    assert!(saved_stdout >= 0, "dup(stdout) failed");
    assert_eq!(
        unsafe { dup2(capture_file.as_raw_fd(), STDOUT_FD) },
        STDOUT_FD,
        "redirect stdout failed"
    );
    let redirect = StdoutRedirect { saved_stdout };

    for &value in values {
        unsafe {
            driver(value);
        }
    }

    drop(redirect);
    drop(capture_file);
    let output = fs::read(&capture_path).expect("read captured stdout");
    fs::remove_file(&capture_path).expect("remove stdout capture file");
    output
}

fn test_values() -> Vec<c_int> {
    let mut values = vec![
        0,
        1,
        -1,
        c_int::MIN,
        c_int::MAX,
        0x0000_007f,
        0x0000_0080,
        0x0000_00ff,
        0x0000_0100,
        -128,
        -129,
        0x0101_0101,
        0x7f7f_7f7f,
        0x8080_8080_u32 as c_int,
        0xffff_0000_u32 as c_int,
        0x0000_ffff,
        0xaaaa_aaaa_u32 as c_int,
        0x5555_5555,
        0x0123_4567,
        0x89ab_cdef_u32 as c_int,
    ];

    // Fixed-seed xorshift64* supplies reproducible values spanning all 32 bits.
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..10_000 {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let random_bits = state.wrapping_mul(0x2545_f491_4f6c_dd1d);
        values.push((random_bits as u32) as c_int);
    }

    values
}

#[test]
fn driver_matches_c_for_boundaries_and_randomized_inputs() {
    let (c_path, rust_path) = library_paths();
    assert_library_exists(&c_path);
    assert_library_exists(&rust_path);

    let values = test_values();
    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared library");
        let rust_library = Library::new(&rust_path).expect("load Rust shared library");
        let c_driver: Symbol<'_, Driver> =
            c_library.get(b"driver\0").expect("load C driver symbol");
        let rust_driver: Symbol<'_, Driver> = rust_library
            .get(b"driver\0")
            .expect("load Rust driver symbol");

        let c_output = capture_driver_output(*c_driver, &values);
        let rust_output = capture_driver_output(*rust_driver, &values);

        let bytes_per_call = (2 * std::mem::size_of::<c_int>()) + 1;
        assert_eq!(c_output.len(), values.len() * bytes_per_call);
        assert_eq!(rust_output, c_output);
    }
}
