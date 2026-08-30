use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::ptr;

type Driver = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn rust_library_path(manifest_dir: &Path) -> PathBuf {
    manifest_dir.join("target/release/libdriver.so")
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    const STDOUT_FILENO: c_int = 1;
    let capture_path =
        std::env::temp_dir().join(format!("driver-differential-{}", std::process::id()));
    let mut capture_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&capture_path)
        .expect("create stdout capture file");

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "flush stdout before redirect");

        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(
            dup2(capture_file.as_raw_fd(), STDOUT_FILENO),
            STDOUT_FILENO,
            "redirect stdout"
        );

        call();

        assert_eq!(fflush(ptr::null_mut()), 0, "flush redirected stdout");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "close saved stdout");
    }

    let mut output = Vec::new();
    capture_file
        .seek(SeekFrom::Start(0))
        .expect("rewind stdout capture");
    capture_file
        .read_to_end(&mut output)
        .expect("read redirected stdout");
    drop(capture_file);
    fs::remove_file(capture_path).expect("remove stdout capture file");
    output
}

fn test_values() -> Vec<c_int> {
    let mut values = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -65_536,
        -256,
        -255,
        -2,
        -1,
        0,
        1,
        2,
        15,
        16,
        255,
        256,
        65_535,
        65_536,
        c_int::MAX - 1,
        c_int::MAX,
    ];

    // Xorshift64 with a fixed nonzero seed gives a reproducible spread of all
    // c_int bit patterns without adding a random-number dependency.
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..10_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        values.push(state as u32 as c_int);
    }
    values
}

#[test]
fn driver_matches_for_every_c_int_shape() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library_path = manifest_dir
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .expect("locate C shared library");
    let rust_library_path = rust_library_path(&manifest_dir)
        .canonicalize()
        .expect("locate Rust shared library");
    assert_ne!(c_library_path, rust_library_path);

    unsafe {
        let c_library = Library::new(&c_library_path).expect("load C shared library");
        let rust_library = Library::new(&rust_library_path).expect("load Rust shared library");
        let c_driver: Symbol<Driver> = c_library.get(b"driver\0").expect("load C driver");
        let rust_driver: Symbol<Driver> = rust_library.get(b"driver\0").expect("load Rust driver");
        assert_ne!(
            *c_driver as *const (), *rust_driver as *const (),
            "both symbols must come from distinct shared libraries"
        );

        let values = test_values();
        let c_output = capture_stdout(|| {
            for &value in &values {
                c_driver(value);
            }
        });
        let rust_output = capture_stdout(|| {
            for &value in &values {
                rust_driver(value);
            }
        });

        assert_eq!(c_output, rust_output);
    }
}
