use libloading::{Library, Symbol};
use std::ffi::{c_float, c_int, c_void};
use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

type Driver = unsafe extern "C" fn(c_float);

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FILENO: c_int = 1;
static CAPTURE_ID: AtomicU64 = AtomicU64::new(0);

fn shared_library_paths() -> (PathBuf, PathBuf) {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    (
        crate_root.join("../c_src/build/libdriver.so"),
        crate_root.join("target/release/libdriver.so"),
    )
}

fn capture_stdout(operation: impl FnOnce()) -> Vec<u8> {
    let capture_path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{}.out",
        std::process::id(),
        CAPTURE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let output_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&capture_path)
        .expect("create stdout capture file");

    unsafe {
        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush stdout");
        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(
            dup2(output_file.as_raw_fd(), STDOUT_FILENO),
            STDOUT_FILENO,
            "redirect stdout"
        );

        operation();

        assert_eq!(fflush(std::ptr::null_mut()), 0, "flush captured stdout");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "close duplicate stdout");
    }

    drop(output_file);
    let output = fs::read(&capture_path).expect("read stdout capture");
    fs::remove_file(&capture_path).expect("remove stdout capture");
    output
}

fn input_bits() -> Vec<u32> {
    let mut inputs = vec![
        0x0000_0000, // positive zero
        0x8000_0000, // negative zero
        0x0000_0001, // smallest positive subnormal
        0x007f_ffff, // largest positive subnormal
        0x0080_0000, // smallest positive normal
        0x7f7f_ffff, // largest positive finite
        0x7f80_0000, // positive infinity
        0xff80_0000, // negative infinity
        0x7fc0_0000, // quiet NaN
        0x7f80_0001, // signaling NaN payload
        0xffff_ffff, // negative NaN payload
    ];

    let mut state = 0x6a09_e667_f3bc_c909_u64;
    for _ in 0..16_384 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        inputs.push(state as u32);
    }
    inputs
}

unsafe fn run_driver(library: &Library, inputs: &[u32]) -> Vec<u8> {
    let driver: Symbol<'_, Driver> =
        unsafe { library.get(b"driver\0") }.expect("load the driver export from shared library");
    capture_stdout(|| {
        for &bits in inputs {
            unsafe { driver(f32::from_bits(bits)) };
        }
    })
}

#[test]
fn driver_matches_for_arbitrary_float_representations() {
    let (c_path, rust_path) = shared_library_paths();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );

    let c_library = unsafe { Library::new(&c_path) }.expect("load C shared library");
    let rust_library = unsafe { Library::new(&rust_path) }.expect("load Rust shared library");
    let inputs = input_bits();

    let c_output = unsafe { run_driver(&c_library, &inputs) };
    let rust_output = unsafe { run_driver(&rust_library, &inputs) };

    assert_eq!(c_output.len(), inputs.len() * 9);
    assert_eq!(rust_output.len(), inputs.len() * 9);
    for (index, ((c_line, rust_line), bits)) in c_output
        .chunks_exact(9)
        .zip(rust_output.chunks_exact(9))
        .zip(inputs)
        .enumerate()
    {
        assert_eq!(
            c_line, rust_line,
            "output mismatch at input {index}, bits 0x{bits:08x}"
        );
    }
}
