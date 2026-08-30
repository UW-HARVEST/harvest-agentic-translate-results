use libloading::{Library, Symbol};
use std::ffi::{c_float, c_int, c_void};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_float);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn library_paths() -> (PathBuf, PathBuf) {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        crate_root.join("../c_src/build/libdriver.so"),
        crate_root.join("target/release/libdriver.so"),
    )
}

fn capture_driver_output(library_path: &Path, inputs: &[u32]) -> Vec<u8> {
    assert!(
        library_path.is_file(),
        "shared library does not exist: {}",
        library_path.display()
    );

    let library = unsafe { Library::new(library_path) }
        .unwrap_or_else(|error| panic!("failed to load {}: {error}", library_path.display()));
    let driver: Symbol<'_, Driver> = unsafe { library.get(b"driver\0") }.unwrap_or_else(|error| {
        panic!(
            "failed to load driver from {}: {error}",
            library_path.display()
        )
    });

    let _stdout_guard = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    let capture_path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{}.out",
        std::process::id(),
        if library_path.to_string_lossy().contains("c_src") {
            "c"
        } else {
            "rust"
        }
    ));
    let mut capture = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&capture_path)
        .expect("create stdout capture");

    unsafe {
        assert_eq!(
            fflush(std::ptr::null_mut()),
            0,
            "flush stdout before capture"
        );
    }
    let saved_stdout = unsafe { dup(STDOUT_FILENO) };
    assert!(saved_stdout >= 0, "duplicate stdout");
    assert_eq!(
        unsafe { dup2(capture.as_raw_fd(), STDOUT_FILENO) },
        STDOUT_FILENO,
        "redirect stdout"
    );

    for &bits in inputs {
        unsafe {
            driver(f32::from_bits(bits));
        }
    }

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "close duplicate stdout");
    }

    capture.seek(SeekFrom::Start(0)).expect("rewind capture");
    let mut output = Vec::new();
    capture.read_to_end(&mut output).expect("read capture");
    drop(capture);
    fs::remove_file(capture_path).expect("remove capture");
    output
}

fn input_corpus() -> Vec<u32> {
    let mut inputs = vec![
        0x0000_0000, // positive zero
        0x8000_0000, // negative zero
        0x0000_0001, // minimum positive subnormal
        0x007f_ffff, // maximum positive subnormal
        0x0080_0000, // minimum positive normal
        0x3f80_0000, // 1.0
        0x3f80_0001, // next value above 1.0
        0xbf80_0000, // -1.0
        0x7f7f_ffff, // maximum finite
        0xff7f_ffff, // minimum finite
        0x7f80_0000, // positive infinity
        0xff80_0000, // negative infinity
        0x7fc0_0000, // quiet NaN
        0xffc0_0000, // negative quiet NaN
        0x7f80_0001, // signaling NaN with minimum payload
        0xffff_ffff, // negative NaN with maximum payload
    ];

    let mut state = 0x4d59_5df4_u32;
    for _ in 0..8_192 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        inputs.push(state);
    }
    inputs
}

fn expected_output(inputs: &[u32]) -> Vec<u8> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = Vec::with_capacity(inputs.len() * 9);
    for &bits in inputs {
        for byte in bits.to_ne_bytes() {
            output.push(HEX[usize::from(byte >> 4)]);
            output.push(HEX[usize::from(byte & 0x0f)]);
        }
        output.push(b'\n');
    }
    output
}

#[test]
fn config_1_driver_matches_for_all_float_shapes_and_random_payloads() {
    let inputs = input_corpus();
    let (c_library, rust_library) = library_paths();
    let c_output = capture_driver_output(&c_library, &inputs);
    let rust_output = capture_driver_output(&rust_library, &inputs);

    assert_eq!(c_output, expected_output(&inputs), "C corpus output");
    assert_eq!(rust_output, c_output, "Rust output differs from C");
}
