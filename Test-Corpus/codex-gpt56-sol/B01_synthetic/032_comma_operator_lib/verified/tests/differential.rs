use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::{OpenOptions, remove_file};
use std::io::{self, Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

type Driver = unsafe extern "C" fn(c_int);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old_fd: c_int, new_fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

struct LoadedDrivers {
    _c_library: Library,
    _rust_library: Library,
    c_driver: Driver,
    rust_driver: Driver,
}

impl LoadedDrivers {
    fn load() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c_path = root.join("c_src/build/libdriver.so");
        let rust_path = find_rust_library(root);

        unsafe {
            let c_library = Library::new(&c_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", c_path.display()));
            let rust_library = Library::new(&rust_path)
                .unwrap_or_else(|error| panic!("failed to load {}: {error}", rust_path.display()));
            let c_driver: Driver = {
                let symbol: Symbol<'_, Driver> = c_library
                    .get(b"driver\0")
                    .expect("C library does not export driver");
                *symbol
            };
            let rust_driver: Driver = {
                let symbol: Symbol<'_, Driver> = rust_library
                    .get(b"driver\0")
                    .expect("Rust library does not export driver");
                *symbol
            };

            Self {
                _c_library: c_library,
                _rust_library: rust_library,
                c_driver,
                rust_driver,
            }
        }
    }

    fn compare(&self, x: c_int) -> Vec<u8> {
        let c_output = capture_stdout(self.c_driver, x).expect("failed to capture C output");
        let rust_output =
            capture_stdout(self.rust_driver, x).expect("failed to capture Rust output");
        assert_eq!(rust_output, c_output, "output differs for x={x}");
        c_output
    }
}

fn find_rust_library(root: &Path) -> PathBuf {
    if let Some(path) = std::env::var_os("RUST_DRIVER_SO") {
        return path.into();
    }

    [
        root.join("target/debug/libdriver.so"),
        root.join("target/debug/deps/libdriver.so"),
        root.join("target/release/libdriver.so"),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .expect("Rust cdylib not found; build the crate before running integration tests")
}

fn capture_stdout(driver: Driver, x: c_int) -> io::Result<Vec<u8>> {
    let _guard = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    let path = std::env::temp_dir().join(format!(
        "driver-differential-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let mut output = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)?;

    unsafe {
        if fflush(ptr::null_mut()) != 0 {
            return Err(io::Error::last_os_error());
        }

        let saved_stdout = dup(1);
        if saved_stdout < 0 {
            return Err(io::Error::last_os_error());
        }
        if dup2(output.as_raw_fd(), 1) < 0 {
            let error = io::Error::last_os_error();
            close(saved_stdout);
            return Err(error);
        }

        driver(x);
        let flush_result = fflush(ptr::null_mut());
        let restore_result = dup2(saved_stdout, 1);
        let restore_error = io::Error::last_os_error();
        close(saved_stdout);

        if flush_result != 0 || restore_result < 0 {
            return Err(restore_error);
        }
    }

    output.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes)?;
    drop(output);
    remove_file(path)?;
    Ok(bytes)
}

fn next_random(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 32) as u32
}

#[test]
fn config_1_negative_x_produces_no_output() {
    let drivers = LoadedDrivers::load();
    let mut seed = 0x0123_4567_89ab_cdef;

    for x in [c_int::MIN, -1] {
        assert!(drivers.compare(x).is_empty(), "C emitted output for x={x}");
    }
    for _ in 0..128 {
        let x = -1 - (next_random(&mut seed) % 1_000_000) as c_int;
        assert!(drivers.compare(x).is_empty(), "C emitted output for x={x}");
    }
}

#[test]
fn config_2_zero_x_produces_no_output() {
    let drivers = LoadedDrivers::load();
    assert!(drivers.compare(0).is_empty());
}

#[test]
fn config_3_one_x_produces_one_line() {
    let drivers = LoadedDrivers::load();
    assert_eq!(drivers.compare(1), b"0 0\n");
}

#[test]
fn config_4_positive_x_produces_many_lines() {
    let drivers = LoadedDrivers::load();
    let mut seed = 0xfedc_ba98_7654_3210;

    assert_eq!(drivers.compare(2), b"0 0\n1 2\n");
    for _ in 0..128 {
        let x = 2 + (next_random(&mut seed) % 255) as c_int;
        let output = drivers.compare(x);
        assert!(output.starts_with(b"0 0\n"));
        assert_eq!(
            output.iter().filter(|&&byte| byte == b'\n').count(),
            x as usize
        );
    }
}
