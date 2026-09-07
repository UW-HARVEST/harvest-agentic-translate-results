use libloading::{Library, Symbol};
use std::ffi::{c_int, c_uint, c_void};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

#[repr(C)]
struct Foo {
    bit_fields: c_uint,
    z: c_int,
}

type Driver = unsafe extern "C" fn(c_uint, c_uint, u8, c_int);
type PrintFoo = unsafe extern "C" fn(*const Foo);

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
}

fn c_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/build/libdriver.so")
}

fn rust_library_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

fn assert_libraries_exist() {
    assert!(
        c_library_path().is_file(),
        "C shared library missing at {}",
        c_library_path().display()
    );
    assert!(
        rust_library_path().is_file(),
        "Rust shared library missing at {}",
        rust_library_path().display()
    );
}

unsafe fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let mut pipe_fds = [-1; 2];
    assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);
    assert_eq!(unsafe { pipe(pipe_fds.as_mut_ptr()) }, 0);

    let saved_stdout = unsafe { dup(1) };
    assert!(saved_stdout >= 0);
    assert_eq!(unsafe { dup2(pipe_fds[1], 1) }, 1);

    call();

    assert_eq!(unsafe { fflush(std::ptr::null_mut()) }, 0);
    assert_eq!(unsafe { dup2(saved_stdout, 1) }, 1);
    assert_eq!(unsafe { close(saved_stdout) }, 0);
    assert_eq!(unsafe { close(pipe_fds[1]) }, 0);

    let mut output = Vec::new();
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader.read_to_end(&mut output).unwrap();
    output
}

fn next_random(state: &mut u64) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 16) as u32
}

fn sampled_z(iteration: usize, state: &mut u64) -> i32 {
    match iteration {
        0 => i32::MIN,
        1 => -1,
        2 => 0,
        3 => 1,
        4 => i32::MAX,
        _ => next_random(state) as i32,
    }
}

fn sampled_field(fits: bool, width_limit: u32, iteration: usize, state: &mut u64) -> u32 {
    if fits {
        next_random(state) % width_limit
    } else {
        match iteration {
            0 => width_limit,
            1 => width_limit + 1,
            2 => u32::MAX,
            _ => width_limit.wrapping_add(next_random(state) % (u32::MAX - width_limit)),
        }
    }
}

#[test]
fn all_valid_configuration_rows_match() {
    let _guard = STDOUT_LOCK.lock().unwrap();
    assert_libraries_exist();

    let c_library = unsafe { Library::new(c_library_path()) }.unwrap();
    let rust_library = unsafe { Library::new(rust_library_path()) }.unwrap();
    let c_driver: Symbol<Driver> = unsafe { c_library.get(b"driver\0") }.unwrap();
    let rust_driver: Symbol<Driver> = unsafe { rust_library.get(b"driver\0") }.unwrap();
    let c_print_foo: Symbol<PrintFoo> = unsafe { c_library.get(b"print_foo\0") }.unwrap();
    let rust_print_foo: Symbol<PrintFoo> = unsafe { rust_library.get(b"print_foo\0") }.unwrap();

    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    let mut row = 0;

    for x_fits in [true, false] {
        for y_fits in [true, false] {
            for b in [false, true] {
                row += 1;
                for iteration in 0..256 {
                    let x = sampled_field(x_fits, 4, iteration, &mut state);
                    let y = sampled_field(y_fits, 8, iteration, &mut state);
                    let z = sampled_z(iteration, &mut state);

                    let c_output = unsafe { capture_stdout(|| c_driver(x, y, u8::from(b), z)) };
                    let rust_output =
                        unsafe { capture_stdout(|| rust_driver(x, y, u8::from(b), z)) };

                    assert_eq!(
                        c_output, rust_output,
                        "CONFIGS.md row {row} diverged for x={x}, y={y}, b={b}, z={z}"
                    );
                }
            }
        }
    }
    assert_eq!(row, 8);

    for x in 0..=3_u32 {
        for y in 0..=7_u32 {
            for b in [false, true] {
                for iteration in 0..32 {
                    let unused_bits = next_random(&mut state) & !0x3f;
                    let foo = Foo {
                        bit_fields: unused_bits | x | (y << 2) | ((u32::from(b)) << 5),
                        z: sampled_z(iteration, &mut state),
                    };

                    let c_output = unsafe { capture_stdout(|| c_print_foo(&foo)) };
                    let rust_output = unsafe { capture_stdout(|| rust_print_foo(&foo)) };

                    assert_eq!(
                        c_output, rust_output,
                        "CONFIGS.md row 9 diverged for bits={:#010x}, z={}",
                        foo.bit_fields, foo.z
                    );
                }
            }
        }
    }
}

#[test]
fn null_pointer_child() {
    let Some(library_path) = std::env::var_os("DRIVER_NULL_CHILD") else {
        return;
    };

    let library = unsafe { Library::new(library_path) }.unwrap();
    let print_foo: Symbol<PrintFoo> = unsafe { library.get(b"print_foo\0") }.unwrap();
    unsafe { print_foo(std::ptr::null()) };
}

fn null_pointer_signal(library_path: &Path) -> i32 {
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "null_pointer_child", "--nocapture"])
        .env("DRIVER_NULL_CHILD", library_path)
        .status()
        .unwrap();

    status
        .signal()
        .unwrap_or_else(|| panic!("null call unexpectedly exited with {status}"))
}

#[test]
fn null_pointer_error_surface_matches() {
    let _guard = STDOUT_LOCK.lock().unwrap();
    assert_libraries_exist();

    let c_signal = null_pointer_signal(&c_library_path());
    let rust_signal = null_pointer_signal(&rust_library_path());

    assert_eq!(c_signal, 11, "C null-pointer behavior changed");
    assert_eq!(rust_signal, c_signal, "ERRORS.md row 1 diverged");
}
