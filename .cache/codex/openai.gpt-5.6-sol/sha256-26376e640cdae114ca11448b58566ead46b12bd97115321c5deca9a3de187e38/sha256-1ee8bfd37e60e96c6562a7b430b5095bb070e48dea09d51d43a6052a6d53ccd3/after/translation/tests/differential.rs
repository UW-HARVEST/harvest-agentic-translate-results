use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int);

const STDOUT_FILENO: c_int = 1;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn read(fd: c_int, buffer: *mut c_void, count: usize) -> isize;
}

struct StdoutCapture {
    read_fd: c_int,
    saved_stdout: c_int,
}

impl StdoutCapture {
    fn start() -> Self {
        unsafe {
            assert_eq!(fflush(std::ptr::null_mut()), 0);

            let mut fds = [-1; 2];
            assert_eq!(pipe(fds.as_mut_ptr()), 0);

            let saved_stdout = dup(STDOUT_FILENO);
            assert!(saved_stdout >= 0);
            assert_eq!(dup2(fds[1], STDOUT_FILENO), STDOUT_FILENO);
            assert_eq!(close(fds[1]), 0);

            Self {
                read_fd: fds[0],
                saved_stdout,
            }
        }
    }

    fn finish(mut self) -> Vec<u8> {
        unsafe {
            assert_eq!(fflush(std::ptr::null_mut()), 0);
            assert_eq!(dup2(self.saved_stdout, STDOUT_FILENO), STDOUT_FILENO);
            assert_eq!(close(self.saved_stdout), 0);
            self.saved_stdout = -1;

            let mut output = Vec::new();
            let mut buffer = [0_u8; 128];
            loop {
                let count = read(
                    self.read_fd,
                    buffer.as_mut_ptr().cast::<c_void>(),
                    buffer.len(),
                );
                assert!(count >= 0);
                if count == 0 {
                    break;
                }
                output.extend_from_slice(&buffer[..count as usize]);
            }

            assert_eq!(close(self.read_fd), 0);
            self.read_fd = -1;
            output
        }
    }
}

impl Drop for StdoutCapture {
    fn drop(&mut self) {
        unsafe {
            if self.saved_stdout >= 0 {
                let _ = fflush(std::ptr::null_mut());
                let _ = dup2(self.saved_stdout, STDOUT_FILENO);
                let _ = close(self.saved_stdout);
            }
            if self.read_fd >= 0 {
                let _ = close(self.read_fd);
            }
        }
    }
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../c_src/build/libdriver.so")
        .canonicalize()
        .expect("C shared library must be built before running differential tests")
}

fn rust_library_path() -> PathBuf {
    let test_executable = std::env::current_exe().expect("test executable path");
    let profile_directory = test_executable
        .parent()
        .and_then(Path::parent)
        .expect("target profile directory");
    let same_profile = profile_directory.join("libdriver.so");
    if same_profile.is_file() {
        return same_profile;
    }

    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

unsafe fn load_driver(library: &Library) -> Symbol<'_, Driver> {
    unsafe {
        library
            .get::<Driver>(b"driver\0")
            .expect("driver symbol must be exported")
    }
}

fn capture_call(driver: Driver, floors: c_int) -> Vec<u8> {
    let capture = StdoutCapture::start();
    unsafe { driver(floors) };
    capture.finish()
}

fn full_domain_inputs() -> Vec<c_int> {
    let mut inputs = vec![
        c_int::MIN,
        c_int::MIN + 1,
        -65_536,
        -256,
        -1,
        0,
        1,
        255,
        256,
        65_535,
        c_int::MAX - 1,
        c_int::MAX,
    ];

    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for _ in 0..2_048 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        inputs.push(state as u32 as c_int);
    }
    inputs
}

#[test]
fn driver_matches_for_full_int_domain_sample() {
    let _stdout_guard = STDOUT_LOCK.lock().expect("stdout lock");
    let c_library = unsafe { Library::new(c_library_path()) }.expect("load C library");
    let rust_path = rust_library_path();
    let rust_library = unsafe { Library::new(&rust_path) }
        .unwrap_or_else(|error| panic!("load Rust library {}: {error}", rust_path.display()));
    let c_driver = unsafe { load_driver(&c_library) };
    let rust_driver = unsafe { load_driver(&rust_library) };

    for floors in full_domain_inputs() {
        let c_output = capture_call(*c_driver, floors);
        let rust_output = capture_call(*rust_driver, floors);
        assert_eq!(rust_output, c_output, "stdout differs for driver({floors})");
    }
}
