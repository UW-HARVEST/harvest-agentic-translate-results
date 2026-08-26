use libloading::{Library, Symbol};
use std::io::Read;
use std::os::unix::io::FromRawFd;
use std::sync::Mutex;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver_c.so");
const RUST_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libdriver.so");

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    let _lock = STDOUT_LOCK.lock().unwrap();
    unsafe {
        libc::fflush(std::ptr::null_mut());
        let mut pipe_fds = [0i32; 2];
        assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0);
        let saved = libc::dup(1);
        assert!(saved >= 0);
        libc::dup2(pipe_fds[1], 1);
        libc::close(pipe_fds[1]);

        f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);

        let mut buf = String::new();
        let mut r = std::fs::File::from_raw_fd(pipe_fds[0]);
        libc::fcntl(pipe_fds[0], libc::F_SETFL, libc::O_NONBLOCK);
        let _ = r.read_to_string(&mut buf);
        buf
    }
}

#[repr(C)]
struct FooT {
    bitfield: u32,
    z: i32,
}

impl FooT {
    fn new(x: u32, y: u32, b: bool, z: i32) -> Self {
        FooT {
            bitfield: (x & 0x3) | ((y & 0x7) << 2) | ((b as u32) << 5),
            z,
        }
    }
}

type DriverFn = unsafe extern "C" fn(u32, u32, bool, i32);
type PrintFooFn = unsafe extern "C" fn(*const FooT);

fn load_libs() -> (Library, Library) {
    unsafe {
        (
            Library::new(C_LIB).expect("Failed to load C library"),
            Library::new(RUST_LIB).expect("Failed to load Rust library"),
        )
    }
}

#[test]
fn test_driver_basic() {
    let (c_lib, rust_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<DriverFn> = c_lib.get(b"driver").unwrap();
        let r_fn: Symbol<DriverFn> = rust_lib.get(b"driver").unwrap();

        for &(x, y, b, z) in &[
            (0u32, 0u32, false, 0i32),
            (1, 2, true, 42),
            (3, 7, true, -1),
            (3, 7, false, i32::MAX),
            (0, 0, false, i32::MIN),
            (2, 5, true, 100),
        ] {
            let c_out = capture_stdout(|| c_fn(x, y, b, z));
            let r_out = capture_stdout(|| r_fn(x, y, b, z));
            assert_eq!(c_out, r_out, "driver({x}, {y}, {b}, {z}) mismatch");
        }
    }
}

#[test]
fn test_driver_truncation() {
    let (c_lib, rust_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<DriverFn> = c_lib.get(b"driver").unwrap();
        let r_fn: Symbol<DriverFn> = rust_lib.get(b"driver").unwrap();

        for &(x, y, b, z) in &[
            (4u32, 0u32, false, 0i32),
            (0, 8, false, 0),
            (7, 15, true, -99),
            (255, 255, true, 0),
        ] {
            let c_out = capture_stdout(|| c_fn(x, y, b, z));
            let r_out = capture_stdout(|| r_fn(x, y, b, z));
            assert_eq!(c_out, r_out, "driver({x}, {y}, {b}, {z}) truncation mismatch");
        }
    }
}

#[test]
fn test_print_foo() {
    let (c_lib, rust_lib) = load_libs();
    unsafe {
        let c_fn: Symbol<PrintFooFn> = c_lib.get(b"print_foo").unwrap();
        let r_fn: Symbol<PrintFooFn> = rust_lib.get(b"print_foo").unwrap();

        for &(x, y, b, z) in &[
            (0u32, 0u32, false, 0i32),
            (1, 2, true, 42),
            (3, 7, true, -1),
            (2, 5, false, 999),
        ] {
            let foo = FooT::new(x, y, b, z);
            let c_out = capture_stdout(|| c_fn(&foo as *const FooT));
            let r_out = capture_stdout(|| r_fn(&foo as *const FooT));
            assert_eq!(c_out, r_out, "print_foo(x={x}, y={y}, b={b}, z={z}) mismatch");
        }
    }
}
