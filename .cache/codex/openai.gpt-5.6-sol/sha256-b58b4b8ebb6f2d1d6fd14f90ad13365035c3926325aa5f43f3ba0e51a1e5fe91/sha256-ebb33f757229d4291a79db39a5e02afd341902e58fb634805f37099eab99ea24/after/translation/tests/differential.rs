use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Mutex;

type PrintLine = unsafe extern "C" fn(*const c_char);
type NoArgs = unsafe extern "C" fn();
type Driver = unsafe extern "C" fn(c_int);

const STDOUT_FILENO: c_int = 1;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

unsafe extern "C" {
    fn close(fd: c_int) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(pipefd: *mut c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
}

struct Libraries {
    c: Library,
    rust: Library,
}

impl Libraries {
    unsafe fn load() -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let c_path = manifest.join("../c_src/build/libdriver.so");
        let rust_path = manifest.join("target/release/libdriver.so");

        assert_shared_object(&c_path);
        assert_shared_object(&rust_path);

        Self {
            c: unsafe { Library::new(c_path).expect("load C shared library") },
            rust: unsafe { Library::new(rust_path).expect("load Rust shared library") },
        }
    }

    unsafe fn pair<T>(&self, name: &[u8]) -> (Symbol<'_, T>, Symbol<'_, T>) {
        (
            unsafe { self.c.get(name).expect("load C symbol") },
            unsafe { self.rust.get(name).expect("load Rust symbol") },
        )
    }
}

fn assert_shared_object(path: &Path) {
    assert!(
        path.is_file(),
        "missing {}; build both release shared libraries first",
        path.display()
    );
}

fn capture_stdout(call: impl FnOnce()) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().expect("stdout lock poisoned");
    let mut pipe_fds = [-1; 2];

    unsafe {
        assert_eq!(fflush(ptr::null_mut()), 0, "flush before capture");
        assert_eq!(pipe(pipe_fds.as_mut_ptr()), 0, "create stdout pipe");

        let saved_stdout = dup(STDOUT_FILENO);
        assert!(saved_stdout >= 0, "duplicate stdout");
        assert_eq!(
            dup2(pipe_fds[1], STDOUT_FILENO),
            STDOUT_FILENO,
            "redirect stdout"
        );
        assert_eq!(close(pipe_fds[1]), 0, "close duplicate pipe writer");

        call();

        assert_eq!(fflush(ptr::null_mut()), 0, "flush captured output");
        assert_eq!(
            dup2(saved_stdout, STDOUT_FILENO),
            STDOUT_FILENO,
            "restore stdout"
        );
        assert_eq!(close(saved_stdout), 0, "close saved stdout");

        let mut output = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let count = read(
                pipe_fds[0],
                chunk.as_mut_ptr().cast::<c_void>(),
                chunk.len(),
            );
            assert!(count >= 0, "read captured stdout");
            if count == 0 {
                break;
            }
            output.extend_from_slice(&chunk[..count as usize]);
        }
        assert_eq!(close(pipe_fds[0]), 0, "close pipe reader");
        output
    }
}

fn compare_call(c_call: impl FnOnce(), rust_call: impl FnOnce()) {
    let c_output = capture_stdout(c_call);
    let rust_output = capture_stdout(rust_call);
    assert_eq!(rust_output, c_output);
}

#[derive(Clone, Copy)]
struct XorShift64(u64);

impl XorShift64 {
    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
}

#[test]
fn config_1_print_line_non_null_randomized() {
    let libraries = unsafe { Libraries::load() };
    let (c_print_line, rust_print_line) = unsafe { libraries.pair::<PrintLine>(b"printLine\0") };
    let mut rng = XorShift64(0x4d59_5df4_d0f3_3173);

    for case in 0..256_usize {
        let length = match case {
            0 => 0,
            1 => 1,
            2 => 4096,
            _ => (rng.next_u64() as usize) % 513,
        };
        let bytes: Vec<u8> = (0..length)
            .map(|_| ((rng.next_u64() % 255) + 1) as u8)
            .collect();
        let value = CString::new(bytes).expect("generated string has no NUL");

        compare_call(
            || unsafe { c_print_line(value.as_ptr()) },
            || unsafe { rust_print_line(value.as_ptr()) },
        );
    }
}

#[test]
fn config_2_bad_direct() {
    let libraries = unsafe { Libraries::load() };
    let (c_bad, rust_bad) = unsafe { libraries.pair::<NoArgs>(b"bad\0") };
    compare_call(|| unsafe { c_bad() }, || unsafe { rust_bad() });
}

#[test]
fn config_3_good_direct() {
    let libraries = unsafe { Libraries::load() };
    let (c_good, rust_good) = unsafe { libraries.pair::<NoArgs>(b"good\0") };
    compare_call(|| unsafe { c_good() }, || unsafe { rust_good() });
}

#[test]
fn config_4_driver_zero() {
    let libraries = unsafe { Libraries::load() };
    let (c_driver, rust_driver) = unsafe { libraries.pair::<Driver>(b"driver\0") };
    compare_call(|| unsafe { c_driver(0) }, || unsafe { rust_driver(0) });
}

#[test]
fn config_5_driver_nonzero_randomized() {
    let libraries = unsafe { Libraries::load() };
    let (c_driver, rust_driver) = unsafe { libraries.pair::<Driver>(b"driver\0") };
    let mut rng = XorShift64(0xa076_1d64_78bd_642f);
    let mut values = vec![c_int::MIN, -1, 1, c_int::MAX];

    while values.len() < 512 {
        let value = rng.next_i32();
        if value != 0 {
            values.push(value);
        }
    }

    for value in values {
        compare_call(
            || unsafe { c_driver(value) },
            || unsafe { rust_driver(value) },
        );
    }
}

#[test]
fn error_1_print_line_null_is_a_noop() {
    let libraries = unsafe { Libraries::load() };
    let (c_print_line, rust_print_line) = unsafe { libraries.pair::<PrintLine>(b"printLine\0") };

    let c_output = capture_stdout(|| unsafe { c_print_line(ptr::null()) });
    let rust_output = capture_stdout(|| unsafe { rust_print_line(ptr::null()) });

    assert_eq!(c_output, Vec::<u8>::new());
    assert_eq!(rust_output, c_output);
}
