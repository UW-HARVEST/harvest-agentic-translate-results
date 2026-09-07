use libloading::Library;
use std::ffi::{c_float, c_int, c_long, c_void};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_float);

const STDOUT_FILENO: c_int = 1;
const SEEK_SET: c_int = 0;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fclose(stream: *mut c_void) -> c_int;
    fn ferror(stream: *mut c_void) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fileno(stream: *mut c_void) -> c_int;
    fn fread(ptr: *mut c_void, size: usize, count: usize, stream: *mut c_void) -> usize;
    fn fseek(stream: *mut c_void, offset: c_long, origin: c_int) -> c_int;
    fn tmpfile() -> *mut c_void;
}

struct RedirectedStdout {
    stream: *mut c_void,
    saved_stdout: c_int,
    restored: bool,
}

impl RedirectedStdout {
    unsafe fn start() -> Self {
        assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);

        let stream = unsafe { tmpfile() };
        assert!(!stream.is_null(), "tmpfile failed");

        let stream_fd = unsafe { fileno(stream) };
        assert!(stream_fd >= 0, "fileno failed");

        let saved_stdout = unsafe { dup(STDOUT_FILENO) };
        assert!(saved_stdout >= 0, "dup(stdout) failed");

        assert_eq!(
            unsafe { dup2(stream_fd, STDOUT_FILENO) },
            STDOUT_FILENO,
            "dup2(capture, stdout) failed"
        );

        Self {
            stream,
            saved_stdout,
            restored: false,
        }
    }

    unsafe fn finish(mut self) -> Vec<u8> {
        unsafe { self.restore_stdout() };
        assert_eq!(
            unsafe { fseek(self.stream, 0, SEEK_SET) },
            0,
            "fseek failed"
        );

        let mut output = Vec::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let read = unsafe { fread(buffer.as_mut_ptr().cast(), 1, buffer.len(), self.stream) };
            output.extend_from_slice(&buffer[..read]);
            if read < buffer.len() {
                assert_eq!(unsafe { ferror(self.stream) }, 0, "fread failed");
                break;
            }
        }

        assert_eq!(unsafe { fclose(self.stream) }, 0, "fclose failed");
        self.stream = std::ptr::null_mut();
        output
    }

    unsafe fn restore_stdout(&mut self) {
        if self.restored {
            return;
        }

        assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);
        assert_eq!(
            unsafe { dup2(self.saved_stdout, STDOUT_FILENO) },
            STDOUT_FILENO,
            "dup2(saved stdout, stdout) failed"
        );
        assert_eq!(unsafe { close(self.saved_stdout) }, 0, "close failed");
        self.restored = true;
    }
}

impl Drop for RedirectedStdout {
    fn drop(&mut self) {
        unsafe {
            if !self.restored {
                let _ = fflush(std::ptr::null_mut());
                let _ = dup2(self.saved_stdout, STDOUT_FILENO);
                let _ = close(self.saved_stdout);
                self.restored = true;
            }
            if !self.stream.is_null() {
                let _ = fclose(self.stream);
                self.stream = std::ptr::null_mut();
            }
        }
    }
}

fn shared_object_paths() -> (PathBuf, PathBuf) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    (
        manifest_dir.join("../c_src/build/libdriver.so"),
        manifest_dir.join("target/release/libdriver.so"),
    )
}

fn capture_driver_output(driver: Driver, inputs: &[u32]) -> Vec<u8> {
    let _lock = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    unsafe {
        let capture = RedirectedStdout::start();
        for &bits in inputs {
            driver(f32::from_bits(bits));
        }
        capture.finish()
    }
}

fn test_inputs() -> Vec<u32> {
    let mut inputs = vec![
        0x0000_0000, // +0
        0x8000_0000, // -0
        0x3f80_0000, // +1
        0xbf80_0000, // -1
        0x0000_0001, // least positive subnormal
        0x007f_ffff, // greatest positive subnormal
        0x0080_0000, // least positive normal
        0x7f7f_ffff, // greatest positive finite
        0x8080_0000, // least negative normal by magnitude
        0xff7f_ffff, // greatest negative finite by magnitude
        0x7f80_0000, // +infinity
        0xff80_0000, // -infinity
        0x7fc0_0000, // canonical quiet NaN
        0xffc0_0000, // negative quiet NaN
        0x7f80_0001, // signaling NaN with minimum payload
        0xff80_0001, // negative signaling NaN with minimum payload
        0x7fff_ffff, // NaN with maximum positive payload
        0xffff_ffff, // NaN with maximum negative payload
        0x0102_0304, // distinct bytes expose byte-order mistakes
        0xff00_aa55,
    ];

    let mut state = 0x6d2b_79f5_u32;
    for _ in 0..25_000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        inputs.push(state);
    }
    inputs
}

#[test]
fn driver_matches_c_for_boundary_and_random_float_representations() {
    let (c_path, rust_path) = shared_object_paths();
    assert!(c_path.is_file(), "missing C shared library: {c_path:?}");
    assert!(
        rust_path.is_file(),
        "missing Rust shared library: {rust_path:?}; run cargo build --release first"
    );

    unsafe {
        let c_library = Library::new(&c_path).expect("load C shared library");
        let rust_library = Library::new(&rust_path).expect("load Rust shared library");
        let c_driver: Driver = *c_library.get(b"driver\0").expect("load C driver");
        let rust_driver: Driver = *rust_library.get(b"driver\0").expect("load Rust driver");

        let inputs = test_inputs();
        let c_output = capture_driver_output(c_driver, &inputs);
        let rust_output = capture_driver_output(rust_driver, &inputs);

        assert_eq!(c_output.len(), inputs.len() * 9);
        assert_eq!(rust_output, c_output);
    }
}
