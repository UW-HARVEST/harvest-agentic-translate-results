use libloading::{Library, Symbol};
use std::ffi::{c_float, c_int, c_void};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

type Driver = unsafe extern "C" fn(c_float);

const STDOUT_FILENO: c_int = 1;
const OUTPUT_BYTES_PER_CALL: usize = 9;

static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
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
            dup2(self.saved_fd, STDOUT_FILENO);
            close(self.saved_fd);
        }
    }
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _capture_guard = CAPTURE_LOCK.lock().expect("stdout capture lock poisoned");
    let capture_path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{}.out",
        std::process::id(),
        CAPTURE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let mut capture_file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(&capture_path)
        .expect("create stdout capture file");

    unsafe {
        assert_eq!(
            fflush(std::ptr::null_mut()),
            0,
            "flush stdout before capture"
        );
    }
    let saved_fd = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_fd >= 0, "duplicate stdout");
    let restore = StdoutRestore { saved_fd };
    assert_eq!(
        unsafe { dup2(capture_file.as_raw_fd(), STDOUT_FILENO) },
        STDOUT_FILENO,
        "redirect stdout"
    );

    call();

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush captured stdout");
    }
    drop(restore);

    capture_file
        .seek(SeekFrom::Start(0))
        .expect("rewind stdout capture");
    let mut output = Vec::new();
    capture_file
        .read_to_end(&mut output)
        .expect("read stdout capture");
    drop(capture_file);
    std::fs::remove_file(capture_path).expect("remove stdout capture file");
    output
}

fn rust_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("locate integration test executable");
    let manifest_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    let deps_directory = test_executable
        .parent()
        .expect("integration test executable has parent");
    let profile_directory = deps_directory
        .parent()
        .expect("deps directory has profile parent");
    let candidates = [
        profile_directory.join("libdriver.so"),
        deps_directory.join("libdriver.so"),
        manifest_directory.join("target/release/libdriver.so"),
    ];

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| {
            panic!(
                "Rust cdylib was not built in {}, {}, or target/release",
                profile_directory.display(),
                deps_directory.display()
            )
        })
}

fn load_library(path: &Path) -> Library {
    unsafe { Library::new(path) }
        .unwrap_or_else(|error| panic!("load shared library {}: {error}", path.display()))
}

fn test_bit_patterns() -> Vec<u32> {
    let mut values = vec![
        0x0000_0000, // positive zero
        0x8000_0000, // negative zero
        0x0000_0001, // smallest positive subnormal
        0x007f_ffff, // largest positive subnormal
        0x0080_0000, // smallest positive normal
        0x3f80_0000, // 1.0
        0xbf80_0000, // -1.0
        0x7f7f_ffff, // largest finite positive value
        0xff7f_ffff, // largest finite negative magnitude
        0x7f80_0000, // positive infinity
        0xff80_0000, // negative infinity
        0x7fc0_0000, // positive quiet NaN
        0xffc0_0000, // negative quiet NaN
        0x7f80_0001, // positive signaling NaN payload
        0xff80_0001, // negative signaling NaN payload
        0x7fff_ffff, // maximum positive NaN payload
        0xffff_ffff, // maximum negative NaN payload
    ];

    let mut state = 0x6d2b_79f5_u32;
    for _ in 0..16_384 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        values.push(state);
    }
    values
}

#[test]
fn configs_row_1_driver_matches_for_all_tested_float_bit_patterns() {
    let manifest_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    let c_library_path = manifest_directory.join("../c_src/build/libdriver.so");
    assert!(
        c_library_path.is_file(),
        "C shared library missing at {}",
        c_library_path.display()
    );
    let rust_library_path = rust_library_path();

    let c_library = load_library(&c_library_path);
    let rust_library = load_library(&rust_library_path);
    let c_driver: Symbol<'_, Driver> =
        unsafe { c_library.get(b"driver\0") }.expect("load C driver export");
    let rust_driver: Symbol<'_, Driver> =
        unsafe { rust_library.get(b"driver\0") }.expect("load Rust driver export");
    let bit_patterns = test_bit_patterns();

    let c_output = capture_stdout(|| {
        for &bits in &bit_patterns {
            unsafe { c_driver(f32::from_bits(bits)) };
        }
    });
    let rust_output = capture_stdout(|| {
        for &bits in &bit_patterns {
            unsafe { rust_driver(f32::from_bits(bits)) };
        }
    });

    let expected_length = bit_patterns.len() * OUTPUT_BYTES_PER_CALL;
    assert_eq!(
        c_output.len(),
        expected_length,
        "C emitted an unexpected number of bytes"
    );
    assert_eq!(
        rust_output.len(),
        expected_length,
        "Rust emitted an unexpected number of bytes"
    );

    if c_output != rust_output {
        let differing_byte = c_output
            .iter()
            .zip(&rust_output)
            .position(|(c_byte, rust_byte)| c_byte != rust_byte)
            .expect("different outputs must contain a differing byte");
        let call_index = differing_byte / OUTPUT_BYTES_PER_CALL;
        let output_start = call_index * OUTPUT_BYTES_PER_CALL;
        let output_end = output_start + OUTPUT_BYTES_PER_CALL;
        panic!(
            "output mismatch at call {call_index}, input bits {:#010x}: C {:?}, Rust {:?}",
            bit_patterns[call_index],
            &c_output[output_start..output_end],
            &rust_output[output_start..output_end]
        );
    }
}
