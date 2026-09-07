use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(f32);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
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

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_library = crate_dir.join("../c_src/build/libdriver.so");
    let rust_library = std::env::var_os("RUST_DRIVER_SO")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate_dir.join("target/release/libdriver.so"));
    (c_library, rust_library)
}

fn capture_calls(library_path: &Path, inputs: &[f32]) -> Vec<u8> {
    assert!(
        library_path.is_file(),
        "shared library does not exist: {}",
        library_path.display()
    );

    let library = unsafe { Library::new(library_path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", library_path.display()));
    let driver: Symbol<'_, Driver> = unsafe { library.get(b"driver\0") }
        .unwrap_or_else(|error| panic!("failed to resolve driver: {error}"));

    let _lock = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{}.bin",
        std::process::id(),
        if library_path.to_string_lossy().contains("c_src") {
            "c"
        } else {
            "rust"
        }
    ));
    let mut output = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&path)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", path.display()));

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "failed to flush stdout");
        let saved_fd = dup(1);
        assert!(saved_fd >= 0, "failed to duplicate stdout");
        let restore = StdoutRestore { saved_fd };
        assert_eq!(dup2(output.as_raw_fd(), 1), 1, "failed to redirect stdout");

        for &input in inputs {
            driver(input);
        }

        assert_eq!(fflush(std::ptr::null_mut()), 0, "failed to flush output");
        drop(restore);
    }

    output
        .seek(SeekFrom::Start(0))
        .expect("failed to rewind captured output");
    let mut bytes = Vec::new();
    output
        .read_to_end(&mut bytes)
        .expect("failed to read captured output");
    drop(output);
    std::fs::remove_file(&path)
        .unwrap_or_else(|error| panic!("failed to remove {}: {error}", path.display()));
    bytes
}

fn input_corpus() -> Vec<f32> {
    let boundary_bits = [
        0x0000_0000,
        0x8000_0000,
        0x0000_0001,
        0x007f_ffff,
        0x0080_0000,
        0x3f00_0000,
        0x3f80_0000,
        0x4000_0000,
        0x7f7f_ffff,
        0x0080_0001,
        0x8080_0001,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0000,
        0x7fc0_0001,
        0x7fff_ffff,
        0x7f80_0001,
        0xffc0_1234,
    ];
    let mut inputs: Vec<f32> = boundary_bits.into_iter().map(f32::from_bits).collect();

    // Xorshift32 with a fixed nonzero seed gives a reproducible spread over
    // all 32-bit float object representations without an extra dependency.
    let mut state = 0x6d2b_79f5_u32;
    for _ in 0..25_000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        inputs.push(f32::from_bits(state));
    }

    inputs
}

#[test]
fn config_1_driver_matches_for_all_float_shapes() {
    let (c_library, rust_library) = library_paths();
    let inputs = input_corpus();
    let c_output = capture_calls(&c_library, &inputs);
    let rust_output = capture_calls(&rust_library, &inputs);

    assert_eq!(c_output.len(), inputs.len() * 9);
    assert_eq!(rust_output, c_output);
}
