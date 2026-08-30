use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int);

const STDOUT_FILENO: c_int = 1;
const SEEK_SET: c_int = 0;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
    fn ftruncate(fd: c_int, length: i64) -> c_int;
}

struct StdoutRestore(c_int);

impl Drop for StdoutRestore {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            assert_eq!(dup2(self.0, STDOUT_FILENO), STDOUT_FILENO);
            assert_eq!(close(self.0), 0);
        }
    }
}

fn shared_library_paths() -> (PathBuf, PathBuf) {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let c_library = crate_root.join("../c_src/build/libdriver.so");
    let rust_library = crate_root.join("target/release/libdriver.so");

    assert!(
        c_library.is_file(),
        "C shared library is missing: {}",
        c_library.display()
    );
    assert!(
        rust_library.is_file(),
        "Rust shared library is missing: {}; run cargo build --release first",
        rust_library.display()
    );

    (c_library, rust_library)
}

fn capture_stdout(file: &mut File, operation: impl FnOnce()) -> Vec<u8> {
    let _lock = STDOUT_LOCK.lock().expect("stdout capture lock poisoned");
    let stdout_copy = unsafe { dup(STDOUT_FILENO) };
    assert!(stdout_copy >= 0);
    let restore = StdoutRestore(stdout_copy);

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
        assert_eq!(ftruncate(file.as_raw_fd(), 0), 0);
        assert_eq!(lseek(file.as_raw_fd(), 0, SEEK_SET), 0);
        assert_eq!(dup2(file.as_raw_fd(), STDOUT_FILENO), STDOUT_FILENO);
    }

    operation();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0);
    }
    drop(restore);

    file.seek(SeekFrom::Start(0)).expect("seek capture file");
    let mut output = Vec::new();
    file.read_to_end(&mut output).expect("read capture file");
    output
}

fn open_capture_file(name: &str) -> File {
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{name}.out",
        std::process::id()
    ));
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(path)
        .expect("open capture file")
}

fn next_random(state: &mut u64) -> i32 {
    // Fixed-seed SplitMix64 gives reproducible coverage of the full bit domain.
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (value ^ (value >> 31)) as i32
}

#[test]
fn driver_matches_c_for_full_int_domain_samples() {
    let (c_path, rust_path) = shared_library_paths();
    let c_library = unsafe { Library::new(c_path) }.expect("load C shared library");
    let rust_library = unsafe { Library::new(rust_path) }.expect("load Rust shared library");
    let c_driver: Symbol<Driver> = unsafe { c_library.get(b"driver\0") }.expect("load C driver");
    let rust_driver: Symbol<Driver> =
        unsafe { rust_library.get(b"driver\0") }.expect("load Rust driver");

    let mut inputs = vec![
        i32::MIN,
        i32::MIN + 1,
        -301,
        -300,
        -299,
        -151,
        -150,
        -149,
        -1,
        0,
        1,
        149,
        150,
        151,
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut seed = 0x5eed_cafe_f00d_beefu64;
    inputs.extend((0..10_000).map(|_| next_random(&mut seed)));

    let mut c_output = open_capture_file("c");
    let mut rust_output = open_capture_file("rust");
    for input in inputs {
        let expected = capture_stdout(&mut c_output, || unsafe { c_driver(input) });
        assert!(
            !expected.is_empty(),
            "C produced no captured output for x={input}"
        );
        let actual = capture_stdout(&mut rust_output, || unsafe { rust_driver(input) });
        assert_eq!(actual, expected, "output differs for x={input}");
    }
}
